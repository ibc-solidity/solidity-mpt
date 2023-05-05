use anyhow::{bail, Result};
use clap::Parser;
use ethers::prelude::*;
use ethers::utils::hex::ToHex;
use ethers::utils::{keccak256, Ganache, GanacheInstance};
use mpt_bindings::erc20::ERC20;
use mpt_bindings::mpt_proof_helper::MPTProofHelper;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

/// Entry point for mpt-proof-gen.
#[derive(Debug, Parser)]
pub struct Cli {
    #[arg(long = "out", help = "Path to test data output")]
    pub out_dir: Option<PathBuf>,

    #[arg(long = "port", default_value = "8545", help = "Port to ganache binds")]
    pub ganache_port: u16,

    #[arg(
        long = "mnemonic",
        default_value = "",
        help = "mnemonic phrase given to ganache"
    )]
    pub mnemonic: String,

    #[arg(long = "mnemonic-index", default_value = "0", help = "mnemonic index")]
    pub mnemonic_index: u64,

    #[arg(
        long = "transfer-count",
        default_value = "10000",
        help = "erc20 transfer count"
    )]
    pub transfer_count: usize,

    #[arg(long = "test-count", default_value = "100", help = "test count")]
    pub test_count: usize,
}

type Client = SignerMiddleware<Provider<Http>, LocalWallet>;

struct Contracts {
    pub erc20: ERC20<Client>,
    pub mpt_proof_verifier: MPTProofHelper<Client>,
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        if self.transfer_count < self.test_count {
            bail!(
                "`transfer-count` must be greater than `test-count`: {} < {}",
                self.transfer_count,
                self.test_count
            );
        }

        let (_ganache, client) = self.run_ganache().await?;
        let contracts = self.deploy_contracts(client.clone()).await?;

        let mut addrs = self
            .transfer_random(&contracts, self.transfer_count)
            .await?;
        addrs.shuffle(&mut thread_rng());

        self.verify_membership(client.clone(), contracts, &addrs[..self.test_count])
            .await?;
        Ok(())
    }

    async fn run_ganache(&self) -> Result<(GanacheInstance, Arc<Client>)> {
        let ganache = Ganache::new()
            .port(self.ganache_port)
            .mnemonic("abstract vacuum mammal awkward pudding scene penalty purchase dinner depart evoke puzzle")
            .spawn();

        // Get the first wallet managed by ganache
        let wallet: LocalWallet = ganache.keys()[0].clone().into();
        println!(
            "Wallet address: {}",
            wallet.address().encode_hex::<String>()
        );

        let provider = Provider::<Http>::try_from(ganache.endpoint())?;
        let chain_id = provider.get_chainid().await?.as_u64();
        println!("Ganache started with chain_id {chain_id}");

        // Create signer client
        let wallet = wallet.with_chain_id(chain_id);
        let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet.clone()));
        Ok((ganache, client))
    }

    async fn deploy_contracts(&self, client: Arc<Client>) -> Result<Contracts> {
        let verifier_contract = MPTProofHelper::deploy(client.clone(), ())?.send().await?;
        println!(
            "MPTProofHelper Contract address: {}",
            verifier_contract.address().encode_hex::<String>()
        );

        let erc20_contract = ERC20::deploy(client.clone(), (U256::from(u128::MAX),))?
            .send()
            .await?;
        println!(
            "ERC20 Contract address: {}",
            erc20_contract.address().encode_hex::<String>()
        );
        Ok(Contracts {
            erc20: erc20_contract,
            mpt_proof_verifier: verifier_contract,
        })
    }

    async fn transfer_random(
        &self,
        contracts: &Contracts,
        num: usize,
    ) -> Result<Vec<(Address, Option<u128>)>> {
        let mut rng = rand::thread_rng();
        let mut m = HashMap::<Address, Option<u128>>::with_capacity(num);
        for i in 0..num {
            let addr = Address::random_using(&mut rng);
            if i % 10 == 0 {
                assert!(m.insert(addr, None).is_none());
            } else {
                let amount = rng.gen_range(1..u64::MAX);
                contracts.erc20.transfer(addr, amount.into()).send().await?;
                assert!(m.insert(addr, Some(amount.into())).is_none());
            }
        }
        Ok(m.into_iter().map(|t| (t.0, t.1)).collect())
    }

    async fn verify_membership(
        &self,
        client: Arc<Client>,
        contracts: Contracts,
        balances: &[(Address, Option<u128>)],
    ) -> Result<()> {
        for (i, (address, expected_balance)) in balances.iter().enumerate() {
            let bn = client.get_block_number().await?;
            let balance_loc = calc_erc20_balance_location(address.to_owned());
            let balance = contracts
                .erc20
                .balance_of(address.to_owned())
                .call()
                .await?;
            let res = client
                .get_proof(
                    contracts.erc20.address(),
                    vec![balance_loc],
                    Some(bn.into()),
                )
                .await?;
            assert!(
                res.storage_proof.len() == 1 && res.storage_proof.get(0).unwrap().value == balance
            );

            let block = client.get_block(bn).await?;
            assert!(block.is_some());

            let account_proof = build_proof(res.account_proof);
            let storage_proof = build_proof(res.storage_proof.get(0).unwrap().proof.clone());

            let account_root: [u8; 32] = {
                let res = contracts
                    .mpt_proof_verifier
                    .verify_rlp_proof(
                        account_proof,
                        block.unwrap().state_root.0,
                        keccak256(contracts.erc20.address().as_bytes()),
                    )
                    .call()
                    .await?;
                let r = rlp::Rlp::new(&res.0);
                assert!(r.is_list());
                let account_root: U256 = r.at(2)?.as_val()?;
                account_root.into()
            };

            let value = {
                let res = contracts
                    .mpt_proof_verifier
                    .verify_rlp_proof(
                        storage_proof.clone(),
                        account_root,
                        keccak256(balance_loc.as_bytes()),
                    )
                    .call()
                    .await?;

                if let Some(expected) = expected_balance {
                    let balance: U256 = rlp::Rlp::new(&res.0).as_val()?;
                    assert_eq!(balance.as_u128(), *expected);
                } else {
                    assert!(res.0.to_vec().len() == 0);
                }

                res.0
            };

            if self.out_dir.is_some() {
                self.write_to_file(
                    format!("{:03}.json", i + 1),
                    ProofData::new(
                        storage_proof.to_vec(),
                        account_root.to_vec(),
                        keccak256(balance_loc.as_bytes()).to_vec(),
                        value.to_vec(),
                    ),
                )?;
            }
        }

        Ok(())
    }

    fn write_to_file(&self, name: impl Into<String>, data: ProofData) -> Result<(), anyhow::Error> {
        let s = serde_json::to_string_pretty(&data)?;
        let name = name.into();

        let out_path = self.out_dir.as_ref().unwrap().join(name);
        if out_path.exists() {
            bail!(format!("dir '{:?}' already exists", out_path));
        }

        File::create(out_path)?.write_all(s.as_bytes())?;

        Ok(())
    }
}

fn calc_erc20_balance_location(address: Address) -> H256 {
    H256::from(keccak256(
        [H256::from(address).as_bytes(), H256::default().as_bytes()].concat(),
    ))
}

fn build_proof(proof: Vec<Bytes>) -> Bytes {
    let acc_proof: Vec<Vec<u8>> = proof.into_iter().map(|b| b.to_vec()).collect();

    let mut stream = rlp::RlpStream::new();
    stream.begin_list(acc_proof.len());
    for p in acc_proof.iter() {
        stream.append_raw(p, 1);
    }
    stream.out().freeze().into()
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct ProofData {
    /// serialized as RLPList[RLPList]
    #[serde(with = "hex")]
    pub proof: Vec<u8>,
    #[serde(with = "hex")]
    pub root: Vec<u8>,
    #[serde(with = "hex")]
    pub key: Vec<u8>,
    /// if empty, the proof is for non-existence proof
    #[serde(with = "hex")]
    pub value: Vec<u8>,
}

impl ProofData {
    pub fn new(proof: Vec<u8>, root: Vec<u8>, key: Vec<u8>, value: Vec<u8>) -> Self {
        Self {
            proof,
            root,
            key,
            value,
        }
    }
}
