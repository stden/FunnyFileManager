//! Shared syntax for ordered key bindings. Resolution performs no effects.

macro_rules! keymap_modifiers {
    ($modifiers:expr, has($first:ident $(+ $rest:ident)*)) => {
        $modifiers.contains(KeyModifiers::$first)
            $(&& $modifiers.contains(KeyModifiers::$rest))*
    };
    ($modifiers:expr, exact($first:ident $(+ $rest:ident)*)) => {
        $modifiers.bits() == KeyModifiers::$first $(.union(KeyModifiers::$rest))*.bits()
    };
    ($modifiers:expr, without($first:ident $(+ $rest:ident)*)) => {
        !$modifiers.intersects(KeyModifiers::$first $(.union(KeyModifiers::$rest))*)
    };
}

/// Define a total resolver from an ordered list of typed commands.
///
/// `has` allows extra modifiers, `exact` accepts only the named combination,
/// and `without` excludes named modifiers. An unqualified binding ignores
/// modifiers. The first matching binding wins; a final fallback is required.
macro_rules! keymap {
    (
        $key:ident;
        $(
            $code:pat $(if $policy:ident($first:ident $(+ $rest:ident)*))? => $action:expr,
        )+
    ) => {
        match $key.code {
            $(
                $code $(if keymap_modifiers!(
                    $key.modifiers, $policy($first $(+ $rest)*)
                ))? => $action,
            )+
        }
    };
}
