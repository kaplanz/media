//! Database schema.

diesel::table! {
    media (id) {
        id -> Binary,
        kind -> Text,
        created -> BigInt,
        updated -> BigInt,
    }
}

diesel::table! {
    tags (media, label) {
        media -> Binary,
        label -> Text,
    }
}

diesel::table! {
    logs (id) {
        id -> Binary,
        media -> Binary,
        kind -> Text,
        date -> BigInt,
    }
}

diesel::table! {
    books (id) {
        id -> Binary,
        isbn -> Nullable<Text>,
        hcid -> Nullable<BigInt>,
        title -> Text,
        cover -> Nullable<Text>,
        about -> Nullable<Text>,
        color -> Nullable<Text>,
    }
}

diesel::table! {
    films (id) {
        id -> Binary,
        tmdb -> Nullable<BigInt>,
        title -> Text,
        year -> Nullable<BigInt>,
        rating -> Nullable<BigInt>,
    }
}

diesel::table! {
    games (id) {
        id -> Binary,
        title -> Text,
        system -> Nullable<Text>,
        rating -> Nullable<BigInt>,
    }
}

diesel::table! {
    links (id) {
        id -> Binary,
        url -> Text,
        title -> Nullable<Text>,
    }
}

diesel::table! {
    shows (id) {
        id -> Binary,
        tmdb -> Nullable<BigInt>,
        title -> Text,
        year -> Nullable<BigInt>,
        rating -> Nullable<BigInt>,
    }
}

diesel::table! {
    games_systems (id) {
        id -> Binary,
        title -> Text,
        system -> Nullable<Text>,
        region -> Nullable<Text>,
        model -> Nullable<Text>,
        revision -> Nullable<Text>,
        serial -> Nullable<Text>,
        variant -> Nullable<Text>,
        complete -> Bool,
        modified -> Bool,
    }
}

diesel::table! {
    games_copies (id) {
        id -> Binary,
        title -> Nullable<Text>,
        system -> Nullable<Text>,
        region -> Nullable<Text>,
        model -> Nullable<Text>,
        revision -> Nullable<Text>,
        serial -> Nullable<Text>,
        complete -> Bool,
        modified -> Bool,
    }
}

diesel::table! {
    games_copies_ref (copy, game) {
        copy -> Binary,
        game -> Binary,
        idx -> BigInt,
    }
}

diesel::table! {
    games_systems_ref (system, game) {
        system -> Binary,
        game -> Binary,
        idx -> BigInt,
    }
}

diesel::table! {
    games_extras (id) {
        id -> Binary,
        title -> Text,
        system -> Nullable<Text>,
        region -> Nullable<Text>,
        model -> Nullable<Text>,
        revision -> Nullable<Text>,
        serial -> Nullable<Text>,
        variant -> Nullable<Text>,
        complete -> Bool,
        modified -> Bool,
    }
}

diesel::table! {
    games_extras_ref (extra, game) {
        extra -> Binary,
        game -> Binary,
        idx -> BigInt,
    }
}

diesel::joinable!(books -> media (id));
diesel::joinable!(films -> media (id));
diesel::joinable!(games -> media (id));
diesel::joinable!(games_copies_ref -> games (game));
diesel::joinable!(games_extras_ref -> games_extras (extra));
diesel::joinable!(games_systems_ref -> games_systems (system));
diesel::joinable!(games_copies_ref -> games_copies (copy));
diesel::joinable!(links -> media (id));
diesel::joinable!(logs -> media (media));
diesel::joinable!(shows -> media (id));
diesel::joinable!(tags -> media (media));

diesel::allow_tables_to_appear_in_same_query!(
    books,
    films,
    games,
    games_copies,
    games_copies_ref,
    games_extras,
    games_extras_ref,
    games_systems,
    games_systems_ref,
    links,
    logs,
    shows,
    media,
    tags,
);
