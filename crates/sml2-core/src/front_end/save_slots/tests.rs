use super::*;

/// A valid 0x50-byte record: one progress byte, counters, checksum, signature.
fn valid_record(node: u8) -> [u8; SLOT_LEN] {
    let mut record = [0u8; SLOT_LEN];
    record[0] = node;
    record[0x08] = 0x80; // the node's progress byte
    record[0x40] = 3; // coins low
    record[0x41] = 1; // coins high
    record[0x42] = 0x12; // BCD tally
    record[0x43] = 5; // lives
    record[0x44] = 2; // money bags
    record[0x45] = record[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
    record[0x46..0x4A].copy_from_slice(&SAVE_SIGNATURE);
    record
}

/// `$2913`/`$30226`: the template is deliberately invalid — lives 5 but
/// checksum 2, while the sum of the bytes before the checksum is 5.
#[test]
fn the_fresh_template_is_invalid_by_design() {
    let template = fresh_template();
    assert_eq!(template[0..0x40], [0u8; 0x40], "64 zero progress bytes");
    assert_eq!(
        (template[0x40], template[0x41], template[0x42]),
        (0, 0, 0),
        "coins and tally"
    );
    assert_eq!(template[0x43], 5, "lives");
    assert_eq!(template[0x44], 0, "money bags");
    assert_eq!(template[0x45], 2, "the ROM's literal checksum byte");
    assert_eq!(
        template[0x46..0x4A],
        SAVE_SIGNATURE,
        "the four signature bytes"
    );
    assert_eq!(template[0x4A], 0, "+$4A is never written");
    let sum = template[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
    assert_eq!(sum, 5, "the real sum differs from the stored checksum");
    assert!(
        !save_record_valid(&template),
        "$28EC rejects the fresh template"
    );
}

/// `$28D9`: every invalid slot is replaced, every valid one is left alone.
#[test]
fn validate_and_init_replaces_only_invalid_slots() {
    let mut slots = SaveSlots::new();
    let record = valid_record(0x23);
    assert!(save_record_valid(&record));
    assert!(slots.write_slot(1, &record));

    let valid = slots.validate_and_init();
    assert_eq!(valid, [false, true, false]);
    assert_eq!(slots.slot(0), Some(&fresh_template()));
    assert_eq!(slots.slot(1), Some(&record));
    assert_eq!(slots.slot(2), Some(&fresh_template()));
    assert!(slots.valid(1));
    assert!(!slots.valid(0));
    assert!(!slots.valid(2));
}

/// A single changed byte fails `$28EC`; only that slot is reinitialized.
#[test]
fn one_corrupt_byte_reinitializes_only_its_own_slot() {
    let mut slots = SaveSlots::new();
    let a = valid_record(1);
    let mut b = valid_record(2);
    let c = valid_record(3);

    b[0x10] ^= 0x01; // a progress byte, covered by the checksum
    assert!(!save_record_valid(&b));
    slots.write_slot(0, &a);
    slots.write_slot(1, &b);
    slots.write_slot(2, &c);

    let valid = slots.validate_and_init();
    assert_eq!(valid, [true, false, true]);
    assert_eq!(slots.slot(0), Some(&a));
    assert_eq!(slots.slot(1), Some(&fresh_template()));
    assert_eq!(slots.slot(2), Some(&c));
}

/// `$2934`: a produced map record round-trips through slot 2.
#[test]
fn a_map_save_record_round_trips_through_a_slot() {
    let record = crate::map_sim::MapSim::new().save_record(5, 0, 0);
    assert!(save_record_valid(&record));

    let mut slots = SaveSlots::new();
    assert!(slots.write_slot(2, &record));
    assert_eq!(slots.load_slot(2), Some(record));

    slots.validate_and_init();
    assert_eq!(slots.load_slot(2), Some(record), "a valid record survives");
    assert_eq!(
        slots.load_slot(0),
        Some(fresh_template()),
        "an empty slot loads the fresh template, i.e. a new game"
    );
}

/// Out-of-range slot indices are errors, never panics.
#[test]
fn out_of_range_slots_are_rejected() {
    let mut slots = SaveSlots::new();
    let record = valid_record(1);
    assert!(slots.slot(3).is_none());
    assert!(slots.load_slot(3).is_none());
    assert!(!slots.write_slot(3, &record));
    assert!(!slots.erase_slot(3));
    assert!(!slots.valid(3));
}

/// `write_slot` needs a complete 0x50-byte record; `erase_slot` writes the
/// template.
#[test]
fn write_requires_a_full_record_and_erase_writes_the_template() {
    let mut slots = SaveSlots::new();
    let record = valid_record(0x40);
    assert!(!slots.write_slot(0, &record[..0x4F]), "short record");
    assert!(slots.write_slot(0, &record));
    assert!(slots.erase_slot(0));
    assert_eq!(slots.slot(0), Some(&fresh_template()));
    assert!(!slots.valid(0));
}

/// The persisted image is exactly the three 0x50 windows; a short file is
/// zero-padded before `$28D9` runs.
#[test]
fn image_round_trips_and_short_input_is_zero_padded() {
    let mut slots = SaveSlots::new();
    slots.write_slot(0, &valid_record(1));
    slots.write_slot(2, &valid_record(3));
    let image = slots.image();
    assert_eq!(image.len(), SAVE_IMAGE_LEN);
    assert_eq!(SaveSlots::from_image(&image), slots);

    let short = SaveSlots::from_image(&[0xAA; 9]);
    assert_eq!(short.image()[9..], [0u8; SAVE_IMAGE_LEN - 9]);
    let mut short = short;
    assert_eq!(short.validate_and_init(), [false, false, false]);
}
