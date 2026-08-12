#[test]
fn below_and_exact_limits_pass_with_exact_counters() {
    let input = super::BudgetInput::new("é", 2, true);
    for limits in [
        super::BudgetLimits::new(3, 2, 3),
        super::BudgetLimits::strictest([
            super::BudgetLimits::new(99, 9, 99),
            super::BudgetLimits::new(2, 2, 2),
        ]),
    ] {
        let budgeted = super::enforce_budgets(input.clone(), limits).unwrap();
        assert_eq!(budgeted.measured.bytes, 2);
        assert_eq!(budgeted.measured.records, 2);
        assert_eq!(budgeted.measured.token_upper_bound, 2);
        assert_eq!(budgeted.bytes, "é".as_bytes());
    }
}

#[test]
fn each_independent_dimension_refuses_above_boundary_without_truncation() {
    for (limits, dimension) in [
        (super::BudgetLimits::new(1, 9, 9), "bytes"),
        (super::BudgetLimits::new(9, 1, 9), "records"),
        (super::BudgetLimits::new(9, 9, 1), "tokens"),
    ] {
        let error =
            super::enforce_budgets(super::BudgetInput::new("é", 2, false), limits).unwrap_err();
        let message = error.to_string();
        assert!(message.starts_with("BUDGET_EXCEEDED"), "{message}");
        assert!(message.contains(dimension), "{message}");
        assert!(message.contains("measured=2"), "{message}");
    }
}

#[test]
fn protected_overflow_has_distinct_fail_closed_reason() {
    let error = super::enforce_budgets(
        super::BudgetInput::new("required", 1, true),
        super::BudgetLimits::new(1, 1, 1),
    )
    .unwrap_err();
    assert!(error.to_string().starts_with("PROTECTED_BUDGET_EXCEEDED"));
}
