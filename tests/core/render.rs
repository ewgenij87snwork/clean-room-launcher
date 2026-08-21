use crate::core::budget::{BudgetInput, BudgetLimits, enforce_budgets};

#[test]
fn renders_l0_l2_l3_in_fixed_order_with_one_terminal_newline() {
    let input = BudgetInput::from_layers("safety\n\n", "проект", "task\n", 3, true);
    let budgeted = enforce_budgets(input, BudgetLimits::new(1024, 3, 1024)).unwrap();
    let artifacts = super::render(&budgeted).unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["context.md"]
    );
    assert_eq!(
        artifacts["context.md"],
        "# L0\nsafety\n# L2\nпроект\n# L3\ntask\n".as_bytes()
    );
}

#[test]
fn render_is_byte_stable_across_process_locale_and_timezone() {
    let budgeted = enforce_budgets(
        BudgetInput::from_layers("a", "β", "c", 3, false),
        BudgetLimits::new(100, 3, 100),
    )
    .unwrap();
    let first = super::render(&budgeted).unwrap();
    let second = super::render(&budgeted).unwrap();
    assert_eq!(first, second);
}

#[test]
fn render_refuses_missing_required_layers() {
    let budgeted = enforce_budgets(
        BudgetInput::new("x", 1, false),
        BudgetLimits::new(10, 1, 10),
    )
    .unwrap();
    assert!(
        super::render(&budgeted)
            .unwrap_err()
            .to_string()
            .starts_with("MISSING_RENDER_LAYER")
    );
}
