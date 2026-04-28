#![allow(clippy::unwrap_used)]

use oya_lite::lifecycle::types::BeadId;
use proptest::prelude::*;

proptest! {
    #[test]
    fn bead_id_parse_valid_ids_always_succeed(s in "[a-z0-9-]{1,64}") {
        let result = BeadId::parse(&s);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn bead_id_parse_trims_leading_trailing_whitespace(s in "  [a-z0-9-]{1,60}  ") {
        let result = BeadId::parse(&s);
        prop_assert!(result.is_ok());
        let trimmed = s.trim();
        let bead_id = result.unwrap();
        prop_assert_eq!(bead_id.as_str(), trimmed);
    }

    #[test]
    fn bead_id_len_validated_within_sixty_four_char_max(s in "[a-z0-9-]{1,64}") {
        let result = BeadId::parse(&s);
        prop_assert!(result.is_ok());
        prop_assert!(result.unwrap().as_str().len() <= 64);
    }

    #[test]
    fn bead_id_display_preserves_original_identifier(s in "[a-z0-9-]{1,64}") {
        let parsed = BeadId::parse(&s).unwrap();
        let displayed = parsed.to_string();
        prop_assert_eq!(displayed, s.trim());
    }

    #[test]
    fn bead_id_clone_produces_equality(s in "[a-z0-9-]{1,64}") {
        let parsed = BeadId::parse(&s).unwrap();
        let cloned = parsed.clone();
        prop_assert_eq!(parsed, cloned);
    }
}