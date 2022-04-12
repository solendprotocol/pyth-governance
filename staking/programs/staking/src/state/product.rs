use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use std::convert::TryInto;

/// This represents a product
/// Currently we store the last time the product account was updated, the current locked balance
/// and the amount by which the locked balance will change in the next epoch
#[account]
#[derive(Default)]
pub struct ProductMetadata {
    pub bump: u8,
    pub last_update_at: u64,
    pub locked: u64,
    pub delta_locked: i64, // locked = locked + delta_locked for the next epoch
}

impl ProductMetadata {
    // Updates the ProductMedata struct.
    // If no time has passed, doesn't do anything
    // If 1 epoch has passed, locked becomes locked + delta_locked
    // If more than 1 epoch has passed, we can assume that no tokens
    // were locked or unlocked in those epochs (otherwise update would've called already)
    // therefore the logic is the same as the case where 1 epoch has passed
    pub fn update(&mut self, current_epoch: u64) -> Result<()> {
        let n: u64 = current_epoch
            .checked_sub(self.last_update_at)
            .ok_or(error!(ErrorCode::GenericOverflow))?;
        self.last_update_at = current_epoch;
        match n {
            0 => Ok(()),
            _ => {
                self.locked = (self.locked as i64)
                    .checked_add(self.delta_locked)
                    .ok_or(error!(ErrorCode::GenericOverflow))?
                    .try_into()
                    .or(Err(ErrorCode::NegativeBalance))?;
                self.delta_locked = 0;
                Ok(())
            }
        }
    }

    // Updates the aggregate account if it is outdated (current_epoch > last_updated_at) and
    // subtracts amount to delta_locked. This method needs to be called everytime a user requests to create a new position.
    pub fn add_locking(&mut self, amount: u64, current_epoch: u64) -> Result<()> {
        self.update(current_epoch);

        self.delta_locked = self
            .delta_locked
            .checked_add(amount as i64)
            .ok_or(error!(ErrorCode::GenericOverflow))?;
        Ok(())
    }

    // Updates the aggregate account if it is outdated (current_epoch > last_updated_at) and
    // subtracts amount to delta_locked. This method needs to be called everytime a user request to unlock a position.
    pub fn add_unlocking(&mut self, amount: u64, current_epoch: u64) -> Result<()> {
        self.update(current_epoch);

        self.delta_locked = self
            .delta_locked
            .checked_sub(amount as i64)
            .ok_or(error!(ErrorCode::GenericOverflow))?;

        // Locked + delta_locked should never be negative, because that'd mean the balance staked to the product is negative
        if (self.locked as i64)
            .checked_add(self.delta_locked)
            .ok_or(error!(ErrorCode::GenericOverflow))?
            < 0
        {
            return Err(error!(ErrorCode::NegativeBalance));
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::state::product::ProductMetadata;
    #[test]
    fn zero_update() {
        let product = &mut ProductMetadata {
            bump: 0,
            last_update_at: 0,
            locked: 0,
            delta_locked: 0,
        };

        product.update(product.last_update_at + 10);
        assert_eq!(product.last_update_at, 10);
        assert_eq!(product.locked, 0);
        assert_eq!(product.delta_locked, 0);
    }

    #[test]
    fn positive_update() {
        let product = &mut ProductMetadata {
            bump: 0,
            last_update_at: 0,
            locked: 0,
            delta_locked: 0,
        };

        product.add_locking(10);
        assert_eq!(product.last_update_at, 0);
        assert_eq!(product.locked, 0);
        assert_eq!(product.delta_locked, 10);

        product.update(product.last_update_at + 1);

        assert_eq!(product.last_update_at, 1);
        assert_eq!(product.locked, 10);
        assert_eq!(product.delta_locked, 0);
    }

    #[test]
    fn negative_update() {
        let product = &mut ProductMetadata {
            bump: 0,
            last_update_at: 0,
            locked: 30,
            delta_locked: 0,
        };

        product.add_unlocking(30);
        assert_eq!(product.last_update_at, 0);
        assert_eq!(product.locked, 30);
        assert_eq!(product.delta_locked, -30);

        product.update(product.last_update_at + 2);

        assert_eq!(product.last_update_at, 2);
        assert_eq!(product.locked, 0);
        assert_eq!(product.delta_locked, 0);
    }

    #[test]
    fn unlock_bigger_than_locked() {
        let product = &mut ProductMetadata {
            bump: 0,
            last_update_at: 0,
            locked: 30,
            delta_locked: 0,
        };

        assert!(product.add_unlocking(40).is_err());

        product.update(product.last_update_at + 5);

        assert_eq!(product.last_update_at, 5);
        assert_eq!(product.locked, 30);
        assert_eq!(product.delta_locked, 0);

        assert!(product.add_unlocking(40).is_err());
    }
}
