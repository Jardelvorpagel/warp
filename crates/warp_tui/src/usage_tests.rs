use warp::tui_export::ConversationUsageTotals;

use super::*;

fn totals(total_tokens: u64, cost_in_cents: f64) -> ConversationUsageTotals {
    ConversationUsageTotals {
        total_tokens,
        cost_in_cents,
    }
}

#[test]
fn token_count_uses_short_and_long_labels() {
    assert_eq!(format_token_count(4, TokenLabelForm::Short), "4 tok");
    assert_eq!(format_token_count(4, TokenLabelForm::Long), "4 tokens");
    assert_eq!(format_token_count(1, TokenLabelForm::Short), "1 tok");
    assert_eq!(format_token_count(1, TokenLabelForm::Long), "1 token");
    assert_eq!(format_token_count(0, TokenLabelForm::Long), "0 tokens");
}

#[test]
fn token_count_abbreviates_large_counts() {
    assert_eq!(format_token_count(9_999, TokenLabelForm::Short), "9999 tok");
    assert_eq!(format_token_count(10_000, TokenLabelForm::Short), "10k tok");
    assert_eq!(
        format_token_count(12_345, TokenLabelForm::Short),
        "12.3k tok"
    );
    assert_eq!(
        format_token_count(999_000, TokenLabelForm::Short),
        "999k tok"
    );
    assert_eq!(
        format_token_count(1_000_000, TokenLabelForm::Short),
        "1M tok"
    );
    assert_eq!(
        format_token_count(1_234_567, TokenLabelForm::Long),
        "1.2M tokens"
    );
}

#[test]
fn cost_formats_cents_as_dollars() {
    assert_eq!(format_cost(0.0), "$0.00");
    assert_eq!(format_cost(0.4), "$0.00");
    assert_eq!(format_cost(3.2), "$0.03");
    assert_eq!(format_cost(123.0), "$1.23");
    assert_eq!(format_cost(10_000.0), "$100.00");
}

#[test]
fn toggle_flips_entry_between_tokens_and_cost() {
    let toggle = TokenCostToggle::default();
    let usage = totals(4, 3.2);

    assert_eq!(toggle.entry_text(usage), "4 tok");
    toggle.toggle();
    assert_eq!(toggle.entry_text(usage), "$0.03");
    toggle.toggle();
    assert_eq!(toggle.entry_text(usage), "4 tok");
}

#[test]
fn cloned_toggles_share_display_mode() {
    // Render closures capture a clone; a click through the clone must be
    // visible to the view-owned original.
    let toggle = TokenCostToggle::default();
    let clone = toggle.clone();
    let usage = totals(4, 3.2);

    clone.toggle();
    assert_eq!(toggle.entry_text(usage), "$0.03");
}
