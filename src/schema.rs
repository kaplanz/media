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
    games_system (id) {
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
    games_owned (id) {
        id -> Binary,
        game -> Binary,
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

diesel::joinable!(books -> media (id));
diesel::joinable!(films -> media (id));
diesel::joinable!(games -> media (id));
diesel::joinable!(games_owned -> games (game));
diesel::joinable!(links -> media (id));
diesel::joinable!(shows -> media (id));
diesel::joinable!(tags -> media (media));

diesel::allow_tables_to_appear_in_same_query!(
    books,
    films,
    games,
    games_owned,
    links,
    shows,
    media,
    tags,
);
