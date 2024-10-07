use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{
        create as associatedTokenCreate, get_associated_token_address, AssociatedToken,
        Create as CreateAssociate,
    },
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata as Metaplex,
    },
    token::{
        mint_to, set_authority, spl_token, transfer, Mint, MintTo, SetAuthority, Token,
        TokenAccount, Transfer as TokenTransfer,
    },
};
use solana_program::system_instruction;

declare_id!("Ku6EPQycT3R2Y6PGy9cbooj9bNewKhVtMuzhhouomqX");

#[program]
pub mod instapump {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global = &mut ctx.accounts.global;
        // Check if already initialized
        require!(!global.initialized, ProgramError::AlreadyInitialized);
        global.initialized = true;
        global.authority = ctx.accounts.user.key();
        Ok(())
    }

    pub fn set_params(
        ctx: Context<SetParams>,
        withdraw_authority: Pubkey,
        fee_recipient: Pubkey,
        initial_virtual_token_reserves: u64,
        initial_virtual_sol_reserves: u64,
        initial_real_token_reserves: u64,
        token_total_supply: u64,
        fee_basis_points: u16,
        mint_fee_sol: u64,
        trading_fee_creator_percent_sol: u16,
        token_share_creator_percent: u16,
        sol_share_first_buyer_after_raydium: u64,
        sol_share_instapump_after_raydium: u64,
    ) -> Result<()> {
        let global = &mut ctx.accounts.global;

        global.fee_recipient = fee_recipient;
        global.withdraw_authority = withdraw_authority;
        global.initial_virtual_token_reserves = initial_virtual_token_reserves;
        global.initial_virtual_sol_reserves = initial_virtual_sol_reserves;
        global.initial_real_token_reserves = initial_real_token_reserves;
        global.token_total_supply = token_total_supply;
        global.fee_basis_points = fee_basis_points;
        global.mint_fee_sol = mint_fee_sol;
        global.trading_fee_creator_percent_sol = trading_fee_creator_percent_sol;
        global.token_share_creator_percent = token_share_creator_percent;
        global.sol_share_first_buyer_after_raydium = sol_share_first_buyer_after_raydium;
        global.sol_share_instapump_after_raydium = sol_share_instapump_after_raydium;

        emit!(SetParamsEvent {
            withdraw_authority,
            fee_recipient,
            initial_virtual_token_reserves,
            initial_virtual_sol_reserves,
            initial_real_token_reserves,
            token_total_supply,
            fee_basis_points,
            mint_fee_sol
        });

        Ok(())
    }

    pub fn create(
        ctx: Context<Create>,
        name: String,
        symbol: String,
        uri: String,
        post_id: String,
        direct_launch: bool,
    ) -> Result<()> {
        msg!("Starting create function");
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        msg!("Starting create function");
        let global = &ctx.accounts.global;
        msg!("Starting create function");

        //////////////////////////////////////////
        // START: Transfer FEE from user to admin
        //////////////////////////////////////////
        let ix = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.fee_recipient.key(),
            global.mint_fee_sol,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.fee_recipient.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        msg!("Complete : Transfer FEE from user to admin");
        //////////////////////////////////////////
        // END: Transfer FEE from user to admin
        //////////////////////////////////////////

        bonding_curve.mint = ctx.accounts.mint.key();
        bonding_curve.virtual_token_reserves = global.initial_virtual_token_reserves;
        bonding_curve.virtual_sol_reserves = global.initial_virtual_sol_reserves;
        bonding_curve.real_token_reserves = global.initial_real_token_reserves;
        bonding_curve.real_sol_reserves = 0;
        bonding_curve.token_total_supply = global.token_total_supply;
        bonding_curve.complete = false;
        bonding_curve.creator_address = ctx.accounts.user.key();

        //////////////////////////////////////////
        // START: Mint Token to Bonding Curve
        //////////////////////////////////////////
        let binding = ctx.accounts.mint.key();
        let seeds = &[
            "mint_authority".as_bytes(),
            binding.as_ref(),
            &[ctx.bumps.mint_authority],
        ];
        let signer = [&seeds[..]];

        // Calculate token amounts
        let bonding_curve_amount = 1_000_000_000 * 1_000_000; // 1_000 million tokens with 6 decimal places
                                                              // let admin_amount: u64 = 200_000_000 * 1_000_000; // 200 million tokens with 6 decimal places

        // Mint tokens to the associated bonding curve account
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.associated_bonding_curve.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                &signer,
            ),
            bonding_curve_amount,
        )?;
        msg!("Complete : Mint Token to Bonding curve");
        //////////////////////////////////////////
        // END: Mint Token to Bonding curve
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Create Metadata - Information about this Token
        //////////////////////////////////////////
        let token_data = DataV2 {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let metadata_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                mint_authority: ctx.accounts.mint_authority.to_account_info(),
                payer: ctx.accounts.user.to_account_info(),
                update_authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            &signer,
        );

        create_metadata_accounts_v3(
            metadata_ctx,
            token_data,
            false, // is_mutable
            true,  // update_authority_is_signer
            None,  // collection_details
        )?;
        msg!("Complete : Create Metadata - Information about this Token");
        //////////////////////////////////////////
        // END: Create Metadata - Information about this Token
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Disable Future minting
        //////////////////////////////////////////
        set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    current_authority: ctx.accounts.mint_authority.to_account_info(),
                    account_or_mint: ctx.accounts.mint.to_account_info(),
                },
                &[&[
                    b"mint_authority",
                    ctx.accounts.mint.key().as_ref(),
                    &[ctx.bumps.mint_authority],
                ]],
            ),
            spl_token::instruction::AuthorityType::MintTokens,
            None,
        )?;
        msg!("Complete : Disable Future minting");
        //////////////////////////////////////////
        // END: Disable Future minting
        //////////////////////////////////////////

        // Emit the creation event
        emit!(CreateEvent {
            name,
            symbol,
            uri,
            post_id,
            mint: ctx.accounts.mint.key(),
            bonding_curve: ctx.accounts.bonding_curve.key(),
            user: ctx.accounts.user.key(),
            direct_launch
        });

        Ok(())
    }

    pub fn buy(ctx: Context<Buy>, amount: u64, max_sol_cost: u64) -> Result<()> {
        // Extract necessary information before mutable borrow
        let bonding_curve_key = ctx.accounts.bonding_curve.key();
        let user_key = ctx.accounts.user.key();
        let fee_recipient_key = ctx.accounts.fee_recipient.key();

        let global = &ctx.accounts.global;

        // Check if the bonding curve is complete
        require!(
            !ctx.accounts.bonding_curve.complete,
            ProgramError::BondingCurveComplete
        );

        // Calculate the SOL cost for the purchase
        let (price_per_token, sol_cost, new_virtual_token_reserves, new_virtual_sol_reserves) =
            calculate_price_and_sol(
                amount,
                ctx.accounts.bonding_curve.virtual_token_reserves,
                ctx.accounts.bonding_curve.virtual_sol_reserves,
                ctx.accounts.bonding_curve.real_token_reserves,
                ctx.accounts.bonding_curve.real_sol_reserves,
            )?;
        msg!("buy() amount: {:?} & sol_cost: {:?}", amount, sol_cost);
        // Check if the SOL cost is within the user's specified limit
        require!(sol_cost <= max_sol_cost, ProgramError::TooMuchSolRequired);

        // Calculate fee
        let admin_fee = (sol_cost * global.fee_basis_points as u64) / 10000;
        let creator_fee = (sol_cost * global.trading_fee_creator_percent_sol as u64) / 10000;
        //////////////////////////////////////////
        // START: Transfer 1% FEE from user to admin
        //////////////////////////////////////////
        let ix = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.fee_recipient.key(),
            admin_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.fee_recipient.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        //////////////////////////////////////////
        // END: Transfer 1% FEE from user to admin
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer {trading_fee_creator_percent_sol}% FEE from user to creator
        //////////////////////////////////////////
        let ix = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.creator.key(),
            creator_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        //////////////////////////////////////////
        // END: Transfer {trading_fee_creator_percent_sol}% FEE from user to creator
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer SOL from user to bonding-curve
        //////////////////////////////////////////
        let transfer_to_bonding_curve_ix =
            system_instruction::transfer(&user_key, &bonding_curve_key, sol_cost);
        anchor_lang::solana_program::program::invoke(
            &transfer_to_bonding_curve_ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.bonding_curve.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        //////////////////////////////////////////
        // END: Transfer SOL from user to bonding-curve
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer Tokens from bonding-curve to user
        //////////////////////////////////////////
        let binding = ctx.accounts.mint.key();
        let seeds = &[
            b"bonding_curve",
            binding.as_ref(),
            &[ctx.bumps.bonding_curve],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = TokenTransfer {
            from: ctx.accounts.associated_bonding_curve.to_account_info(),
            to: ctx.accounts.associated_user.to_account_info(),
            authority: ctx.accounts.bonding_curve.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        transfer(cpi_ctx, amount)?;
        //////////////////////////////////////////
        // END: Transfer Tokens from bonding-curve to user
        //////////////////////////////////////////

        let new_real_token_reserves = ctx
            .accounts
            .bonding_curve
            .real_token_reserves
            .checked_sub(amount)
            .unwrap();
        let new_real_sol_reserves = ctx
            .accounts
            .bonding_curve
            .real_sol_reserves
            .checked_add(sol_cost)
            .unwrap();

        // Update bonding curve state
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        bonding_curve.virtual_token_reserves = new_virtual_token_reserves;
        bonding_curve.virtual_sol_reserves = new_virtual_sol_reserves;
        bonding_curve.real_token_reserves = new_real_token_reserves;
        bonding_curve.real_sol_reserves = new_real_sol_reserves;
        //////////////////////////////////////////
        // END: Update quantity tracking variables
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: update bonding curve first buyer
        //////////////////////////////////////////
        if (bonding_curve.first_buyer_address == Pubkey::default()) {
            bonding_curve.first_buyer_address = user_key;
        }
        //////////////////////////////////////////
        // END: update bonding curve first buyer
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Set bonding-curve to Complete; if it is complete
        //////////////////////////////////////////
        if (bonding_curve.real_token_reserves == 0) {
            bonding_curve.complete = true;
            emit!(CompleteEvent {
                mint: ctx.accounts.mint.key(),
                bonding_curve: bonding_curve.key(),
                timestamp: Clock::get()?.unix_timestamp,
            });
        }
        //////////////////////////////////////////
        // END: Set bonding-curve to Complete
        //////////////////////////////////////////

        // Emit the trade event
        emit!(TradeEvent {
            mint: ctx.accounts.mint.key(),
            sol_amount: sol_cost,
            token_amount: amount,
            is_buy: true,
            user: user_key,
            timestamp: Clock::get()?.unix_timestamp,
            virtual_sol_reserves: new_virtual_sol_reserves,
            virtual_token_reserves: new_virtual_token_reserves,
        });

        Ok(())
    }
    pub fn sell(ctx: Context<Sell>, amount: u64, min_sol_output: u64) -> Result<()> {
        let bonding_curve_key = ctx.accounts.bonding_curve.key();
        let user_key = ctx.accounts.user.key();
        let fee_recipient_key = ctx.accounts.fee_recipient.key();
        let mint_key = ctx.accounts.mint.key();

        // Check if the bonding curve is complete
        require!(
            !ctx.accounts.bonding_curve.complete,
            ProgramError::BondingCurveComplete
        );

        // Calculate values
        let (price_per_token, sol_output, new_virtual_token_reserves, new_virtual_sol_reserves) =
            calculate_price_and_sol_sell_operation(
                amount,
                ctx.accounts.bonding_curve.virtual_token_reserves,
                ctx.accounts.bonding_curve.virtual_sol_reserves,
                ctx.accounts.bonding_curve.real_token_reserves,
                ctx.accounts.bonding_curve.real_sol_reserves,
            )?;
        msg!("sell() amount: {:?} & sol_cost: {:?}", amount, sol_output);
        // require!(sol_output >= min_sol_output, ProgramError::SlippageExceeded);

        // Calculate fee
        let admin_fee = (sol_output * ctx.accounts.global.fee_basis_points as u64) / 10000;
        let creator_fee =
            (sol_output * ctx.accounts.global.trading_fee_creator_percent_sol as u64) / 10000;
        //////////////////////////////////////////
        // START: Transfer 1% FEE from user to admin
        //////////////////////////////////////////
        let ix = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.fee_recipient.key(),
            admin_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.fee_recipient.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        //////////////////////////////////////////
        // END: Transfer 1% FEE from user to admin
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer {trading_fee_creator_percent_sol}% FEE from user to creator
        //////////////////////////////////////////
        let ix = system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.creator.key(),
            creator_fee,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.creator.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        //////////////////////////////////////////
        // END: Transfer {trading_fee_creator_percent_sol}% FEE from user to creator
        //////////////////////////////////////////

        // Perform transfers
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TokenTransfer {
                    from: ctx.accounts.associated_user.to_account_info(),
                    to: ctx.accounts.associated_bonding_curve.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        let bonding_curve = &mut ctx.accounts.bonding_curve;

        // Transfer SOL from bonding curve to user
        **bonding_curve.to_account_info().try_borrow_mut_lamports()? -= sol_output;
        **ctx.accounts.user.try_borrow_mut_lamports()? += sol_output;

        //////////////////////////////////////////
        // START: Update quantity tracking variables
        //
        // new_virtual_TOKEN_reserves = virtual_token_reserves + token_amount
        //
        // new_virtual_SOL_reserves = virtual_sol_reserves - sol_cost
        //
        // new_real_TOKEN_reserves = real_token_reserves + token_amount
        //
        // new_real_SOL_reserves = real_sol_reserves - sol_cost
        //////////////////////////////////////////
        // let new_virtual_token_reserves = bonding_curve
        //     .virtual_token_reserves
        //     .checked_add(amount)
        //     .unwrap();
        // let new_virtual_sol_reserves = bonding_curve
        //     .virtual_sol_reserves
        //     .checked_sub(sol_output)
        //     .unwrap();
        bonding_curve.virtual_token_reserves = new_virtual_token_reserves;

        bonding_curve.virtual_sol_reserves = new_virtual_sol_reserves;

        bonding_curve.real_token_reserves = bonding_curve
            .real_token_reserves
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        bonding_curve.real_sol_reserves = bonding_curve
            .real_sol_reserves
            .checked_sub(sol_output)
            .ok_or(ProgramError::InsufficientFunds)?;
        //////////////////////////////////////////
        // END: Update quantity tracking variables
        //////////////////////////////////////////

        // Emit the trade event
        emit!(TradeEvent {
            mint: ctx.accounts.mint.key(),
            sol_amount: sol_output,
            token_amount: amount,
            is_buy: false,
            user: user_key,
            timestamp: Clock::get()?.unix_timestamp,
            virtual_sol_reserves: bonding_curve.virtual_sol_reserves,
            virtual_token_reserves: bonding_curve.virtual_token_reserves,
        });
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let global = &ctx.accounts.global;
        let mint_key = ctx.accounts.mint.key();
        // Check if the caller is the admin
        require!(
            ctx.accounts.user.key() == global.withdraw_authority,
            ProgramError::NotAuthorized
        );

        // Calculate the minimum balance required for rent exemption
        let rent = Rent::get()?;
        let minimum_balance = rent.minimum_balance(8 + BondingCurve::LEN);

        // calculate token withdraw metrics
        let current_token_balance = ctx.accounts.associated_bonding_curve.amount;
        let creator_token_share =
            (current_token_balance * global.token_share_creator_percent as u64) / 10000;
        let token_amount_admin_withdraw = current_token_balance.saturating_sub(creator_token_share);

        // Calculate the amount to withdraw, ensuring we leave enough for rent exemption
        let current_sol_balance = ctx.accounts.bonding_curve.to_account_info().lamports();
        let sol_amount = current_sol_balance.saturating_sub(minimum_balance + 10000);
        let sol_amount_admin_withdraw = sol_amount
            .saturating_sub(global.sol_share_first_buyer_after_raydium)
            .saturating_sub(global.sol_share_instapump_after_raydium);

        msg!("withdraw() current_sol_balance: {:?}", current_sol_balance);
        msg!(
            "withdraw() sol_amount_admin_withdraw: {:?}",
            sol_amount_admin_withdraw
        );
        require!(
            sol_amount_admin_withdraw > 0,
            ProgramError::InsufficientFunds
        );

        let seeds = &[
            b"bonding_curve",
            mint_key.as_ref(),
            &[ctx.bumps.bonding_curve],
        ];
        let signer = &[&seeds[..]];

        //////////////////////////////////////////
        // START: Transfer Token share creator (token_share_creator_percent)
        //////////////////////////////////////////
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TokenTransfer {
                    from: ctx.accounts.associated_bonding_curve.to_account_info(),
                    to: ctx.accounts.creator_associated_user.to_account_info(),
                    authority: ctx.accounts.bonding_curve.to_account_info(),
                },
                signer,
            ),
            creator_token_share,
        )?;
        msg!("Token transferred to admin");
        //////////////////////////////////////////
        // END: Transfer Token share creator (token_share_creator_percent)
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer left Token to admin
        //////////////////////////////////////////
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TokenTransfer {
                    from: ctx.accounts.associated_bonding_curve.to_account_info(),
                    to: ctx.accounts.associated_user.to_account_info(),
                    authority: ctx.accounts.bonding_curve.to_account_info(),
                },
                signer,
            ),
            token_amount_admin_withdraw,
        )?;
        msg!("Token transferred to admin");
        //////////////////////////////////////////
        // END: Transfer left Token to admin
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer SOL share first buyer (sol_share_first_buyer_after_raydium)
        //////////////////////////////////////////
        **ctx
            .accounts
            .bonding_curve
            .to_account_info()
            .try_borrow_mut_lamports()? -= global.sol_share_first_buyer_after_raydium;
        **ctx
            .accounts
            .first_buyer_address
            .to_account_info()
            .try_borrow_mut_lamports()? += global.sol_share_first_buyer_after_raydium;
        //////////////////////////////////////////
        // END: Transfer SOL share first buyer (sol_share_first_buyer_after_raydium)
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer SOL share instapump (sol_share_instapump_after_raydium)
        //////////////////////////////////////////
        **ctx
            .accounts
            .bonding_curve
            .to_account_info()
            .try_borrow_mut_lamports()? -= global.sol_share_instapump_after_raydium;
        **ctx
            .accounts
            .fee_recipient
            .to_account_info()
            .try_borrow_mut_lamports()? += global.sol_share_instapump_after_raydium;
        //////////////////////////////////////////
        // END: Transfer SOL share instapump (sol_share_instapump_after_raydium)
        //////////////////////////////////////////

        //////////////////////////////////////////
        // START: Transfer left SOL to admin
        //////////////////////////////////////////
        **ctx
            .accounts
            .bonding_curve
            .to_account_info()
            .try_borrow_mut_lamports()? -= sol_amount;
        **ctx
            .accounts
            .user
            .to_account_info()
            .try_borrow_mut_lamports()? += sol_amount;
        //////////////////////////////////////////
        // END: Transfer left SOL to admin
        //////////////////////////////////////////

        // Update bonding curve state
        let bonding_curve = &mut ctx.accounts.bonding_curve;
        bonding_curve.real_sol_reserves = 0;
        bonding_curve.real_token_reserves = 0;

        emit!(WithdrawEvent {
            mint: mint_key,
            sol_amount: sol_amount_admin_withdraw,
            token_amount: token_amount_admin_withdraw,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + Global::LEN,
        seeds = [b"global"],
        bump
    )]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetParams<'info> {
    #[account(
        mut, 
        seeds = [b"global"], 
        bump,
        constraint = global.authority == user.key() @ ProgramError::NotAuthorized
        )]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(name: String, symbol: String, uri: String, post_id: String, direct_launch: bool)]
pub struct Create<'info> {
    #[account(
        init,
        payer = user,
        mint::decimals = 6,
        mint::authority = mint_authority.key()
    )]
    pub mint: Account<'info, Mint>,

    /// CHECK: This is safe as we're using it as a seed for PDA
    #[account(seeds = [b"mint_authority", mint.key().as_ref()], bump)]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = user,
        space = 8,
        seeds = [b"instagram_post", post_id.as_bytes()],
        bump,
    )]
    pub instapump_post_account: UncheckedAccount<'info>,

    #[account(seeds = [b"global"], bump)]
    pub global: Box<Account<'info, Global>>,

    #[account(mut, constraint = global.fee_recipient == fee_recipient.key())]
    pub fee_recipient: UncheckedAccount<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + BondingCurve::LEN,
        seeds = [b"bonding_curve", mint.key().as_ref()],
        bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    pub associated_bonding_curve: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub associated_user: Account<'info, TokenAccount>,

    // #[account(
    //     mut
    //     // init_if_needed,
    //     // payer = user,
    //     // associated_token::mint = mint,
    //     // associated_token::authority = fee_recipient,
    //     // address = get_associated_token_address(&fee_recipient.key(), &mint.key())
    // )]
    // pub fee_recipient_token_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(seeds = [b"event_authority"], bump)]
    /// CHECK: This account is only used as a PDA for event emission
    pub event_authority: UncheckedAccount<'info>,

    pub token_metadata_program: Program<'info, Metaplex>,
    #[account(mut)]
    /// CHECK: This account is checked in the instruction
    pub metadata: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    #[account(mut, constraint = global.fee_recipient == fee_recipient.key())]
    pub fee_recipient: UncheckedAccount<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"bonding_curve", mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Account<'info, BondingCurve>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    #[account(mut, constraint = bonding_curve.creator_address == creator.key())]
    pub creator: UncheckedAccount<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub associated_user: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(seeds = [b"global"], bump)]
    pub global: Account<'info, Global>,
    #[account(
        mut,
        seeds = [b"bonding_curve", mint.key().as_ref()],
        bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,

    #[account(mut, constraint = global.fee_recipient == fee_recipient.key())]
    pub fee_recipient: UncheckedAccount<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    pub associated_bonding_curve: Account<'info, TokenAccount>,

    #[account(mut, constraint = bonding_curve.creator_address == creator.key())]
    pub creator: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub associated_user: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"bonding_curve", mint.key().as_ref()],
        bump
    )]
    pub bonding_curve: Account<'info, BondingCurve>,
    pub global: Account<'info, Global>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = bonding_curve,
    )]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub associated_user: Account<'info, TokenAccount>,
    #[account(mut, constraint = global.fee_recipient == fee_recipient.key())]
    pub fee_recipient: UncheckedAccount<'info>,
    #[account(mut, constraint = bonding_curve.first_buyer_address == first_buyer_address.key())]
    pub first_buyer_address: UncheckedAccount<'info>,
    #[account(mut, constraint = bonding_curve.creator_address == creator.key())]
    pub creator: UncheckedAccount<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = creator,
    )]
    pub creator_associated_user: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct Global {
    pub initialized: bool,
    pub authority: Pubkey,
    pub withdraw_authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u16,
    pub mint_fee_sol: u64,
    pub trading_fee_creator_percent_sol: u16,
    pub token_share_creator_percent: u16,
    pub sol_share_first_buyer_after_raydium: u64, // not in percent
    pub sol_share_instapump_after_raydium: u64,   // not in percent
}

impl Global {
    pub const LEN: usize = 1 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8;
}

#[account]
#[derive(Default)]
pub struct BondingCurve {
    pub mint: Pubkey,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub creator_address: Pubkey,
    pub first_buyer_address: Pubkey,
}

impl BondingCurve {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 8 + 8 + 1 + 32 + 32;
}

#[error_code]
pub enum ProgramError {
    #[msg("The given account is not authorized to execute this instruction.")]
    NotAuthorized,
    #[msg("The program is already initialized.")]
    AlreadyInitialized,
    #[msg("The provided fee recipient token account does not match the global fee recipient")]
    InvalidFeeRecipient,
    #[msg("slippage: Too much SOL required to buy the given amount of tokens.")]
    TooMuchSolRequired,
    #[msg("slippage: Too little SOL received to sell the given amount of tokens.")]
    TooLittleSolReceived,
    #[msg("The mint does not match the bonding curve.")]
    MintDoesNotMatchBondingCurve,
    #[msg("The bonding curve has completed and liquidity migrated to raydium.")]
    BondingCurveComplete,
    #[msg("The bonding curve has not completed.")]
    BondingCurveNotComplete,
    #[msg("The program is not initialized.")]
    NotInitialized,
    #[msg("The program is facing InsufficientFunds.")]
    InsufficientFunds,
    #[msg("The program is facing ArithmeticOverflow.")]
    ArithmeticOverflow,
    #[msg("The program is facing SlippageExceeded.")]
    SlippageExceeded,
    #[msg("The program is facing InsufficientTokens.")]
    InsufficientTokens,
    #[msg("This Insta post ID has already been used to create a token.")]
    PostIdAlreadyUsed,
}

// Event definitions
#[event]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub post_id: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub direct_launch: bool,
}

#[event]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
}

#[event]
pub struct CompleteEvent {
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct SetParamsEvent {
    pub withdraw_authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u16,
    pub mint_fee_sol: u64,
}

fn calculate_price_and_sol(
    token_amount: u64,
    virtual_token_reserves: u64,
    virtual_sol_reserves: u64,
    real_token_reserves: u64,
    real_sol_reserves: u64,
) -> Result<(u64, u64, u64, u64)> {
    const SCALING_FACTOR: u64 = 100; // For testing, set to 1 when going live
                                     // Ensure we're not trying to buy more tokens than available
    require!(
        token_amount <= real_token_reserves,
        ProgramError::InsufficientTokens
    );

    // Calculate the constant product k
    let k = (virtual_token_reserves as u128) * (virtual_sol_reserves as u128);

    // Calculate new virtual token reserves after purchase
    let new_virtual_token_reserves = virtual_token_reserves
        .checked_sub(token_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Calculate new virtual SOL reserves
    let new_virtual_sol_reserves = (k / new_virtual_token_reserves as u128) as u64;

    // Calculate SOL required for purchase
    let sol_required = new_virtual_sol_reserves
        .checked_sub(virtual_sol_reserves)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Apply scaling factor
    let scaled_sol_required = sol_required / SCALING_FACTOR;

    // Calculate the average price per token
    let price_per_token = scaled_sol_required
        .checked_mul(1_000_000) // Scale up for precision (assuming 6 decimal places for tokens)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(token_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok((
        price_per_token,
        scaled_sol_required,
        new_virtual_token_reserves,
        new_virtual_sol_reserves,
    ))
}

fn calculate_price_and_sol_sell_operation(
    token_amount: u64,
    virtual_token_reserves: u64,
    virtual_sol_reserves: u64,
    real_token_reserves: u64,
    real_sol_reserves: u64,
) -> Result<(u64, u64, u64, u64)> {
    const SCALING_FACTOR: u64 = 100; // For testing, set to 1 when going live

    // Calculate the constant product k
    let k = (virtual_token_reserves as u128) * (virtual_sol_reserves as u128);

    // Calculate new virtual token reserves after sell
    let new_virtual_token_reserves = virtual_token_reserves
        .checked_add(token_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Calculate new virtual SOL reserves
    let new_virtual_sol_reserves = (k / new_virtual_token_reserves as u128) as u64;

    // Calculate SOL recieved on sell
    let sol_required = virtual_sol_reserves
        .checked_sub(new_virtual_sol_reserves)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Apply scaling factor
    let scaled_sol_required = sol_required / SCALING_FACTOR;

    // Calculate the average price per token
    let price_per_token = scaled_sol_required
        .checked_mul(1_000_000) // Scale up for precision (assuming 6 decimal places for tokens)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(token_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok((
        price_per_token,
        scaled_sol_required,
        new_virtual_token_reserves,
        new_virtual_sol_reserves,
    ))
}
