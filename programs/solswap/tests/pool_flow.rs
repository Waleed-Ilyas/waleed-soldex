use anchor_lang::solana_program::program_option::COption;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::{AccountDeserialize, AccountSerialize, InstructionData, Space, ToAccountMetas};
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::spl_token::state::{Account as SplAccount, AccountState, Mint};
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
use solana_program_test::{processor, ProgramTest, ProgramTestContext};
use solana_sdk::account::Account;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

fn solswap_processor<'program, 'accounts, 'info, 'data>(
    program_id: &'program Pubkey,
    accounts: &'accounts [anchor_lang::solana_program::account_info::AccountInfo<'info>],
    instruction_data: &'data [u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult {
    // SAFETY: Anchor's entrypoint handles the account infos synchronously and does not retain them.
    let (accounts, instruction_data) = unsafe {
        (
            std::mem::transmute::<
                &'accounts [anchor_lang::solana_program::account_info::AccountInfo<'info>],
                &'program [anchor_lang::solana_program::account_info::AccountInfo<'program>],
            >(accounts),
            std::mem::transmute::<&'data [u8], &'program [u8]>(instruction_data),
        )
    };
    solswap::entry(program_id, accounts, instruction_data)
}

fn mint_account() -> Account {
    let state = Mint {
        mint_authority: COption::None,
        supply: 100,
        decimals: 0,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    let mut data = vec![0; Mint::LEN];
    Mint::pack(state, &mut data).unwrap();
    Account {
        lamports: 1_000_000,
        data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    }
}

fn token_account(mint: Pubkey, owner: Pubkey) -> Account {
    let state = SplAccount {
        mint,
        owner,
        amount: 1_000,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    let mut data = vec![0; SplAccount::LEN];
    SplAccount::pack(state, &mut data).unwrap();
    Account {
        lamports: 1_000_000,
        data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    }
}

async fn send_instruction(
    context: &mut ProgramTestContext,
    instruction: Instruction,
    signer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let blockhash = context.banks_client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, signer],
        blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(())
}

#[tokio::test]
async fn liquidity_swap_and_withdraw_update_token_and_pool_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let user = Keypair::new();
    let mint_a = Keypair::new().pubkey();
    let mint_b = Keypair::new().pubkey();
    let (mint_a, mint_b) = if mint_a.to_bytes() < mint_b.to_bytes() {
        (mint_a, mint_b)
    } else {
        (mint_b, mint_a)
    };
    let (pool, bump) =
        Pubkey::find_program_address(&[b"pool", mint_a.as_ref(), mint_b.as_ref()], &solswap::ID);
    let position_id = 7_u64;
    let (position, _) = Pubkey::find_program_address(
        &[
            b"position",
            pool.as_ref(),
            user.pubkey().as_ref(),
            &position_id.to_le_bytes(),
        ],
        &solswap::ID,
    );
    let vault_a = get_associated_token_address(&pool, &mint_a);
    let vault_b = get_associated_token_address(&pool, &mint_b);

    let mut pool_data = Vec::new();
    solswap::Pool {
        authority: user.pubkey(),
        mint_a,
        mint_b,
        reserve_a: 0,
        reserve_b: 0,
        total_liquidity: 0,
        fee_bps: 30,
        bump,
    }
    .try_serialize(&mut pool_data)?;
    pool_data.resize(8 + solswap::Pool::INIT_SPACE, 0);

    let mut test = ProgramTest::new("solswap", solswap::ID, processor!(solswap_processor));
    test.add_program(
        "spl_token",
        TOKEN_PROGRAM_ID,
        processor!(spl_token::processor::Processor::process),
    );
    test.add_program(
        "spl_associated_token_account",
        anchor_spl::associated_token::ID,
        processor!(spl_associated_token_account::processor::process_instruction),
    );
    test.add_account(
        user.pubkey(),
        Account {
            lamports: 2_000_000_000,
            data: Vec::new(),
            owner: anchor_lang::system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    test.add_account(mint_a, mint_account());
    test.add_account(mint_b, mint_account());
    test.add_account(
        pool,
        Account {
            lamports: 1_000_000,
            data: pool_data,
            owner: solswap::ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    test.add_account(vault_a, token_account(mint_a, pool));
    test.add_account(vault_b, token_account(mint_b, pool));
    test.add_account(
        get_associated_token_address(&user.pubkey(), &mint_a),
        token_account(mint_a, user.pubkey()),
    );
    test.add_account(
        get_associated_token_address(&user.pubkey(), &mint_b),
        token_account(mint_b, user.pubkey()),
    );

    let mut context = test.start_with_context().await;
    let instruction = Instruction {
        program_id: solswap::ID,
        accounts: solswap::accounts::AddLiquidity {
            user: user.pubkey(),
            mint_a,
            mint_b,
            pool,
            position,
            user_a: get_associated_token_address(&user.pubkey(), &mint_a),
            user_b: get_associated_token_address(&user.pubkey(), &mint_b),
            vault_a,
            vault_b,
            token_program: TOKEN_PROGRAM_ID,
            system_program: anchor_lang::system_program::ID,
        }
        .to_account_metas(None),
        data: solswap::instruction::AddLiquidity {
            position_id,
            amount_a: 500,
            amount_b: 500,
            minimum_shares: 0,
        }
        .data(),
    };

    send_instruction(&mut context, instruction, &user).await?;
    let pool_account = context
        .banks_client
        .get_account(pool)
        .await?
        .expect("pool state should exist");
    let mut pool_data = pool_account.data.as_slice();
    let pool_state = solswap::Pool::try_deserialize(&mut pool_data)?;
    assert_eq!(pool_state.reserve_a, 500);
    assert_eq!(pool_state.reserve_b, 500);
    assert_eq!(pool_state.total_liquidity, 500);

    let swap_instruction = Instruction {
        program_id: solswap::ID,
        accounts: solswap::accounts::Swap {
            user: user.pubkey(),
            mint_a,
            mint_b,
            pool,
            user_a: get_associated_token_address(&user.pubkey(), &mint_a),
            user_b: get_associated_token_address(&user.pubkey(), &mint_b),
            vault_a,
            vault_b,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: solswap::instruction::Swap {
            amount_in: 100,
            minimum_out: 80,
            a_to_b: true,
        }
        .data(),
    };
    send_instruction(&mut context, swap_instruction, &user).await?;

    let remove_instruction = Instruction {
        program_id: solswap::ID,
        accounts: solswap::accounts::RemoveLiquidity {
            owner: user.pubkey(),
            mint_a,
            mint_b,
            pool,
            position,
            user_a: get_associated_token_address(&user.pubkey(), &mint_a),
            user_b: get_associated_token_address(&user.pubkey(), &mint_b),
            vault_a,
            vault_b,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: solswap::instruction::RemoveLiquidity {
            minimum_a: 600,
            minimum_b: 417,
        }
        .data(),
    };
    send_instruction(&mut context, remove_instruction, &user).await?;

    let user_a_account = context
        .banks_client
        .get_account(get_associated_token_address(&user.pubkey(), &mint_a))
        .await?
        .expect("user token account should remain");
    let user_a_state = SplAccount::unpack(&user_a_account.data)?;
    let user_b_account = context
        .banks_client
        .get_account(get_associated_token_address(&user.pubkey(), &mint_b))
        .await?
        .expect("user token account should remain");
    let user_b_state = SplAccount::unpack(&user_b_account.data)?;
    assert_eq!(user_a_state.amount, 1_000);
    assert_eq!(user_b_state.amount, 1_000);

    let pool_account = context
        .banks_client
        .get_account(pool)
        .await?
        .expect("pool state should remain after withdrawal");
    let mut pool_data = pool_account.data.as_slice();
    let pool_state = solswap::Pool::try_deserialize(&mut pool_data)?;
    assert_eq!(pool_state.reserve_a, 0);
    assert_eq!(pool_state.reserve_b, 0);
    assert_eq!(pool_state.total_liquidity, 0);
    assert!(context.banks_client.get_account(position).await?.is_none());

    Ok(())
}
