use bdk::{
    bitcoin::{Address, Network},
    wallet::AddressIndex,
    SignOptions, Wallet,
};
use bdk_esplora::esplora_client;
use bdk_esplora::EsploraExt;
use bdk_file_store::KeychainStore;
use std::str::FromStr;

const SEND_AMOUNT: u64 = 5000;
const STOP_GAP: usize = 50;
const PARALLEL_REQUESTS: usize = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::temp_dir().join("bdk-esplora-example");
    let db = KeychainStore::new_from_path(db_path)?;
    let external_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/0/*)";
    let internal_descriptor = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/0'/0'/1/*)";

    let mut wallet = Wallet::new(
        external_descriptor,
        Some(internal_descriptor),
        db,
        Network::Testnet,
    )?;

    let address = wallet.get_address(AddressIndex::New);
    println!("Generated Address: {}", address);

    let balance = wallet.get_balance();
    println!("Wallet balance before syncing: {} sats", balance.total());

    println!("Syncing...");
    // Scanning the chain...
    let esplora_url = "https://mempool.space/testnet/api";
    let client = esplora_client::Builder::new(esplora_url).build_blocking()?;
    let spks = wallet.spks_of_all_keychains();
    let checkpoints = wallet.checkpoints();
    let update = client.scan(
        checkpoints,
        spks,
        core::iter::empty(),
        core::iter::empty(),
        STOP_GAP,
        PARALLEL_REQUESTS,
    )?;
    wallet.apply_update(update)?;
    wallet.commit()?;

    let balance = wallet.get_balance();
    println!("Wallet balance after syncing: {} sats", balance.total());

    if balance.total() == 0 {
        println!("Please send some coins to the receiving address");
        std::process::exit(0);
    }

    let faucet_address = Address::from_str("mkHS9ne12qx9pS9VojpwU5xtRd4T7X7ZUt")?;

    let mut tx_builder = wallet.build_tx();
    tx_builder
        .add_recipient(faucet_address.script_pubkey(), SEND_AMOUNT)
        .enable_rbf();

    let (mut psbt, _) = tx_builder.finish()?;
    let finalized = wallet.sign(&mut psbt, SignOptions::default())?;
    assert!(finalized);

    let tx = psbt.extract_tx();
    client.broadcast(&tx)?;
    println!("Tx broadcasted! Txid: {}", tx.txid());

    Ok(())
}
