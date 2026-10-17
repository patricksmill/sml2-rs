//! Native sound sequencing, channel dispatch, and ordered channel-control outputs.

pub mod driver;
pub mod pulse;

pub use driver::{
    lookup_sfx_pointer, SfxLookupError, SfxSlotState, SoundDriver, MASTER_VOLUME_LOW_TIME,
    MASTER_VOLUME_NORMAL, SLOT0_BUSY_EXCEPTIONS, SLOT0_START_TABLE, SLOT0_TICK_TABLE,
    SLOT1_BUSY_EXCEPTION, SLOT1_START_TABLE, SLOT1_TICK_TABLE, SLOT2_START_TABLE,
    SLOT2_TICK_TABLE,
};
pub use pulse::{PulseEffectState, PulseMusicFlags, PulseMusicState, PulseWrite};
