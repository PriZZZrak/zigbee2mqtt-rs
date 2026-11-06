// Original: .upstream/zigbee-herdsman-converters/test/utils.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - accepts a transaction number reused after the dedup window expires
//   - batteryVoltageToPercentage
//   - boolean keys
//   - detects a duplicate transaction within the dedup window
//   - does not deduplicate when the model sets publishDuplicateTransaction
//   - getFromLookup
//   - getFromLookupByValue
//   - getTransition
//   - gets linear
//   - gets linear with offset
//   - gets non-linear
//   - hasAlreadyProcessedMessage
//   - invalid key types
//   - keeps the last 5 transactions, which can arrive out of order
//   - key not found
//   - mapNumberRange
//   - multiple keys with same value
//   - number keys
//   - should map value (0..100)->(0..200)
//   - should map value (0..100)->(0..200) outside range
//   - should map value (0..100)->(100..0)
//   - should map value (100..200)->(200..100)
//   - should map value (100..200)->(300..400)
//   - should match false with exact match
//   - should match numeric key
//   - should match true with case insensitive lookup
//   - should match true with exact match
//   - should match with case insensitive lookup (lowercase key)
//   - should match with case insensitive lookup (mixed case key)
//   - should match with case insensitive lookup (uppercase key)
//   - should return default value when key not found and default provided
//   - should return default value when value not found and default provided
//   - should return first matching key
//   - should return key when value matches
//   - should return {time: 0, specified: false} if manufacturerID is 4476 and key is brightness and message has color or color_temp
//   - should return {time: 0, specified: false} if manufacturerID is 4476 and key is brightness and message has color_temp
//   - should return {time: 0, specified: false} if neither message.transition nor options.transition is specified
//   - should return {time: 0, specified: false} if options.transition is an empty string
//   - should return {time: 0, specified: false} if options.transition is not specified
//   - should return {time: 100, specified: true} if message.transition is specified
//   - should return {time: 200, specified: true} if options.transition is specified
//   - should round to specified precision
//   - should throw when key is not string or number
//   - should throw when key not found and no default provided
//   - should throw when keyIsBool is true but key is not boolean
//   - should throw when value not found and no default provided
//   - string keys
//   - toNumber
//   - utils
//   - value found
//   - value not found

#[test]
#[ignore = "not implemented yet"]
fn accepts_a_transaction_number_reused_after_the_dedup_window_expires() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn battery_voltage_to_percentage() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn boolean_keys() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn detects_a_duplicate_transaction_within_the_dedup_window() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_deduplicate_when_the_model_sets_publish_duplicate_transaction() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_from_lookup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_from_lookup_by_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_transition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn gets_linear() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn gets_linear_with_offset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn gets_non_linear() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn has_already_processed_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn invalid_key_types() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_the_last_5_transactions_which_can_arrive_out_of_order() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn key_not_found() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn map_number_range() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn multiple_keys_with_same_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn number_keys() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_map_value_0_100_0_200() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_map_value_0_100_0_200_outside_range() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_map_value_0_100_100_0() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_map_value_100_200_200_100() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_map_value_100_200_300_400() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_match_false_with_exact_match() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_match_numeric_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_match_true_with_case_insensitive_lookup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_match_true_with_exact_match() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_match_with_case_insensitive_lookup_lowercase_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_match_with_case_insensitive_lookup_mixed_case_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_match_with_case_insensitive_lookup_uppercase_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_default_value_when_key_not_found_and_default_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_default_value_when_value_not_found_and_default_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_first_matching_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_key_when_value_matches() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_time_0_specified_false_if_manufacturer_id_is_4476_and_key_is_brightness_and_message_has_color_or_color_temp(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_time_0_specified_false_if_manufacturer_id_is_4476_and_key_is_brightness_and_message_has_color_temp(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_time_0_specified_false_if_neither_message_transition_nor_options_transition_is_specified(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_time_0_specified_false_if_options_transition_is_an_empty_string() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_time_0_specified_false_if_options_transition_is_not_specified() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_time_100_specified_true_if_message_transition_is_specified() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_time_200_specified_true_if_options_transition_is_specified() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_round_to_specified_precision() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_when_key_is_not_string_or_number() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_when_key_not_found_and_no_default_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_when_key_is_bool_is_true_but_key_is_not_boolean() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_when_value_not_found_and_no_default_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn string_keys() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn to_number() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn utils() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn value_found() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn value_not_found() {
    todo!()
}
