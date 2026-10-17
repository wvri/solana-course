use anchor_lang::prelude::*;

declare_id!("4kdXLeUUvnWZJqUkYAeKGRvF9LkkR1gvmjd2Mug58D8m");

#[program]
pub mod profile {
    use super::*;

    pub fn create_profile(ctx: Context<CreateProfile>, name: String, bio: String) -> Result<()> {
        require!(name.len() <= 32, ProfileError::NameTooLong);
        require!(bio.len() <= 100, ProfileError::BioTooLong);

        let p: &mut Account<Profile> = &mut ctx.accounts.profile;
        p.authority = ctx.accounts.user.key();
        p.name = name;
        p.bio = bio;
        p.bump = ctx.bumps.profile;
        Ok(())
    }

    pub fn update_profile(ctx: Context<UpdateProfile>, name: String, bio: String) -> Result<()> {
        require!(name.len() <= 32, ProfileError::NameTooLong);
        require!(bio.len() <= 100, ProfileError::BioTooLong);

        let p: &mut Account<Profile> = &mut ctx.accounts.profile;
        p.name = name;
        p.bio = bio;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateProfile<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + 32 + (4 + 32) + (4 + 100) + 1,
        seeds = [b"profile", user.key().as_ref()],
        bump
    )]
    pub profile: Account<'info, Profile>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateProfile<'info> {
    #[account(
        mut,
        seeds = [b"profile", authority.key().as_ref()],
        bump = profile.bump,
        has_one = authority
    )]
    pub profile: Account<'info, Profile>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Profile {
    pub authority: Pubkey,
    pub name: String,
    pub bio: String,
    pub bump: u8,
}

#[error_code]
pub enum ProfileError {
    #[msg("Name is too long (max 32)")]
    NameTooLong,
    #[msg("Bio is too long (max 100)")]
    BioTooLong,
}
