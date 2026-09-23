use super::*;

#[test]
fn env_names_are_portable_and_never_empty() {
    assert_eq!(env_name("Work/API token"), "WORK_API_TOKEN");
    assert_eq!(env_name("___"), "ENTRY");
    assert_eq!(env_name("Crème brûlée"), "CR_ME_BR_L_E");
}

#[test]
fn env_values_escape_shell_sensitive_characters() {
    assert_eq!(
        quote_env_value("a\\b\"c\n${TOKEN}\t"),
        "\"a\\\\b\\\"c\\n\\${TOKEN}\\t\""
    );
}

#[test]
fn duplicate_env_names_receive_a_collision_safe_suffix() {
    let mut used = HashSet::new();
    assert_eq!(unique_env_name("API", &mut used), "KDBX_API_PASSWORD");
    assert_eq!(unique_env_name("API", &mut used), "KDBX_API_2_PASSWORD");
    // A real entry named API_2 must not collide with the generated suffix.
    assert_eq!(unique_env_name("API_2", &mut used), "KDBX_API_2_2_PASSWORD");
}
