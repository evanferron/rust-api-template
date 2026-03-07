// @generated automatically by Diesel CLI.

diesel::table! {
    posts (id) {
        id -> Uuid,
        user_id -> Uuid,
        #[max_length = 255]
        title -> Varchar,
        content -> Text,
        published -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        password -> Varchar,
        #[max_length = 100]
        first_name -> Varchar,
        #[max_length = 100]
        last_name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

// Définition des relations
diesel::joinable!(posts -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(posts, users,);
