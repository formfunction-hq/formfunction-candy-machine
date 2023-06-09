use solana_program_test::{BanksClientError, ProgramTestBanksClientExt, ProgramTestContext};
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::transport::TransportError;
use solana_sdk::{
    account::Account,
    program_pack::Pack,
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    transport,
};
use spl_token::state::Mint;
pub use spl_token::ID as TOKEN_PROGRAM_ID;

use crate::core::{master_edition_manager::MasterEditionManager, metadata_manager};
use crate::utils::SolanaProgramTestResult;

// See this for update_blockhash and the following function:
// https://discord.com/channels/428295358100013066/439194979856809985/1070821035630395462
pub async fn update_blockhash(context: &mut ProgramTestContext) -> transport::Result<()> {
    let client = &mut context.banks_client;
    context.last_blockhash = client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    context
        .warp_to_slot(current_slot + 1)
        .map_err(|_| TransportError::Custom("Warp to slot failed!".to_string()))?;
    Ok(())
}

// Warp to a specific slot. Copied from above to avoid modifying all the usages of
// update_blockhash with a new argument.
pub async fn update_blockhash_to_slot(
    context: &mut ProgramTestContext,
    slot: u64,
) -> transport::Result<()> {
    let client = &mut context.banks_client;
    context.last_blockhash = client
        .get_new_latest_blockhash(&context.last_blockhash)
        .await
        .unwrap();

    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    context
        .warp_to_slot(current_slot + slot)
        .map_err(|_| TransportError::Custom("Warp to slot failed!".to_string()))?;
    Ok(())
}

/// Perform native lamports transfer.
#[allow(dead_code)]
pub async fn transfer_lamports(
    client: &mut ProgramTestContext,
    wallet: &Keypair,
    to: &Pubkey,
    amount: u64,
) -> transport::Result<()> {
    update_blockhash(client).await?;
    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(&wallet.pubkey(), to, amount)],
        Some(&wallet.pubkey()),
        &[wallet],
        client.last_blockhash,
    );

    client.banks_client.process_transaction(tx).await?;

    Ok(())
}

pub async fn get_token_account(
    client: &mut ProgramTestContext,
    token_account: &Pubkey,
) -> transport::Result<spl_token::state::Account> {
    let account = client.banks_client.get_account(*token_account).await?;
    Ok(spl_token::state::Account::unpack(&account.unwrap().data).unwrap())
}

pub async fn get_balance(context: &mut ProgramTestContext, pubkey: &Pubkey) -> u64 {
    context.banks_client.get_balance(*pubkey).await.unwrap()
}

pub async fn get_token_balance(context: &mut ProgramTestContext, token_account: &Pubkey) -> u64 {
    get_token_account(context, token_account)
        .await
        .unwrap()
        .amount
}

pub async fn new_funded_keypair(context: &mut ProgramTestContext, amount: u64) -> Keypair {
    let new_key = Keypair::new();
    airdrop(context, &new_key.pubkey(), amount).await.unwrap();
    new_key
}

pub async fn airdrop(
    context: &mut ProgramTestContext,
    receiver: &Pubkey,
    amount: u64,
) -> transport::Result<()> {
    update_blockhash(context).await?;
    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &context.payer.pubkey(),
            receiver,
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
    Ok(())
}

pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}

pub async fn get_account_if_exists(
    context: &mut ProgramTestContext,
    pubkey: &Pubkey,
) -> Result<Option<Account>, BanksClientError> {
    context
        .banks_client
        .get_account_with_commitment(*pubkey, CommitmentLevel::Processed)
        .await
}

pub async fn get_account(context: &mut ProgramTestContext, pubkey: &Pubkey) -> Account {
    get_account_if_exists(context, pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

pub async fn assert_account_empty(context: &mut ProgramTestContext, pubkey: &Pubkey) {
    let account = context
        .banks_client
        .get_account(*pubkey)
        .await
        .expect("Could not get account!");
    assert_eq!(account, None);
}

#[allow(dead_code)]
pub async fn get_mint(context: &mut ProgramTestContext, pubkey: &Pubkey) -> Mint {
    let account = get_account(context, pubkey).await;
    Mint::unpack(&account.data).unwrap()
}

#[allow(dead_code)]
pub async fn create_token_account(
    context: &mut ProgramTestContext,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
) -> SolanaProgramTestResult {
    update_blockhash(context).await?;
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, account],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .map_err(|e| e.into())
}

pub async fn create_associated_token_account(
    context: &mut ProgramTestContext,
    wallet: &Pubkey,
    token_mint: &Pubkey,
) -> transport::Result<Pubkey> {
    update_blockhash(context).await?;
    let recent_blockhash = context.last_blockhash;

    let tx = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &context.payer.pubkey(),
                wallet,
                token_mint,
                &TOKEN_PROGRAM_ID,
            ),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        recent_blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    Ok(spl_associated_token_account::get_associated_token_address(
        wallet, token_mint,
    ))
}

pub async fn create_mint(
    context: &mut ProgramTestContext,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    decimals: u8,
    mint: Option<Keypair>,
) -> transport::Result<Keypair> {
    update_blockhash(context).await?;
    let mint = mint.unwrap_or_else(Keypair::new);
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                authority,
                freeze_authority,
                decimals,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
    Ok(mint)
}

pub async fn mint_to_wallets(
    context: &mut ProgramTestContext,
    mint_pubkey: &Pubkey,
    authority: &Keypair,
    allocations: Vec<(Pubkey, u64)>,
) -> SolanaProgramTestResult<Vec<Pubkey>> {
    update_blockhash(context).await?;
    let mut atas = Vec::with_capacity(allocations.len());

    #[allow(clippy::needless_range_loop)]
    for i in 0..allocations.len() {
        let ata = create_associated_token_account(context, &allocations[i].0, mint_pubkey).await?;
        mint_tokens(
            context,
            authority,
            mint_pubkey,
            &ata,
            allocations[i].1,
            None,
        )
        .await?;
        atas.push(ata);
    }
    Ok(atas)
}

pub async fn mint_tokens(
    context: &mut ProgramTestContext,
    authority: &Keypair,
    mint: &Pubkey,
    account: &Pubkey,
    amount: u64,
    additional_signer: Option<&Keypair>,
) -> SolanaProgramTestResult {
    update_blockhash(context).await?;
    let mut signing_keypairs = vec![authority, &context.payer];
    if let Some(signer) = additional_signer {
        signing_keypairs.push(signer);
    }

    let ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        account,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.payer.pubkey()),
        &signing_keypairs,
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(tx)
        .await
        .map_err(|e| e.into())
}

#[allow(dead_code)]
pub async fn transfer(
    context: &mut ProgramTestContext,
    mint: &Pubkey,
    from: &Keypair,
    to: &Keypair,
    amount: u64,
) -> SolanaProgramTestResult {
    update_blockhash(context).await?;
    create_associated_token_account(context, &to.pubkey(), mint).await?;
    let tx = Transaction::new_signed_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            &from.pubkey(),
            &to.pubkey(),
            &from.pubkey(),
            &[&from.pubkey()],
            amount,
        )
        .unwrap()],
        Some(&from.pubkey()),
        &[from],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await
        .map_err(|e| e.into())
}

pub async fn prepare_nft(minter: &Keypair) -> MasterEditionManager {
    let nft = metadata_manager::MetadataManager::new(minter);
    MasterEditionManager::new(&nft)
}

pub fn strip_empty_bytes_from_string(str: String) -> String {
    str.trim_matches(char::from(0)).to_string()
}
