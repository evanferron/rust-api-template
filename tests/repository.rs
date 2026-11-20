//! Tests d'intégration des capacités génériques de `db::repository`.
//!
//! Chaque test ouvre sa propre connexion dans une transaction de test
//! (`begin_test_transaction`) : rien n'est commité, les tests sont indépendants
//! et ne nécessitent pas de reset de la base.
//!
//! Requiert `DATABASE_TEST_URL`.

use api::db::post::model::{NewPost, Post};
use api::db::post::repository::PostRepository;
use api::db::repository::prelude::*;
use api::db::schema::{posts, users};
use api::db::user::model::{NewUser, User};
use api::db::user::repository::UserRepository;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use tokio::sync::OnceCell;
use uuid::Uuid;

static MIGRATED: OnceCell<()> = OnceCell::const_new();

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn test_db_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_TEST_URL").expect("DATABASE_TEST_URL must be set")
}

/// Connexion dans une transaction de test (rollback automatique au drop).
async fn test_conn() -> AsyncPgConnection {
    let url = test_db_url();
    MIGRATED
        .get_or_init(|| async {
            api::db::run_migrations(&url).await.expect("run migrations");
        })
        .await;

    let mut conn = AsyncPgConnection::establish(&url)
        .await
        .expect("connect to test DB");
    conn.begin_test_transaction()
        .await
        .expect("begin test transaction");
    conn
}

async fn seed_user(conn: &mut AsyncPgConnection) -> User {
    UserRepository::create(
        conn,
        NewUser {
            id: Uuid::new_v4(),
            email: format!("{}@test.com", Uuid::new_v4()),
            password: "hash".to_string(),
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
        },
    )
    .await
    .unwrap()
}

/// Crée un post avec un `created_at` contrôlé (pour tester l'ordre de pagination).
async fn seed_post(conn: &mut AsyncPgConnection, user_id: Uuid, created_at: DateTime<Utc>) -> Post {
    let post = PostRepository::create(
        conn,
        NewPost {
            id: Uuid::new_v4(),
            user_id,
            title: "title".to_string(),
            content: "content".to_string(),
            published: false,
        },
    )
    .await
    .unwrap();

    diesel::update(posts::table.find(post.id))
        .set(posts::created_at.eq(created_at))
        .get_result::<Post>(conn)
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// Repositories de test — chaque macro est testée isolément
// ---------------------------------------------------------------------------

struct UserExtrasRepo;
api::impl_exists!(UserExtrasRepo, api::db::schema::users, Uuid);
api::impl_count!(UserExtrasRepo, api::db::schema::users);

struct UserPaginateRepo;
api::impl_paginate!(UserPaginateRepo, User, api::db::schema::users, created_at);

// Table temporaire pour SoftDelete (aucune table du template n'a de deleted_at)
diesel::table! {
    soft_items (id) {
        id -> Uuid,
        name -> Text,
        deleted_at -> Nullable<Timestamptz>,
    }
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = soft_items)]
#[allow(dead_code)]
struct SoftItem {
    id: Uuid,
    name: String,
    deleted_at: Option<DateTime<Utc>>,
}

struct SoftItemRepo;
api::impl_soft_delete!(SoftItemRepo, SoftItem, crate::soft_items, Uuid);
api::impl_find_by_id!(SoftItemRepo, SoftItem, crate::soft_items, Uuid);

async fn create_soft_items_table(conn: &mut AsyncPgConnection) {
    diesel::sql_query(
        "CREATE TEMP TABLE soft_items (id UUID PRIMARY KEY, name TEXT NOT NULL, deleted_at TIMESTAMPTZ NULL)",
    )
    .execute(conn)
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------
// FindAll / FindById / Delete (BaseRepository)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn find_by_id_returns_row_or_none() {
    let mut conn = test_conn().await;
    let user = seed_user(&mut conn).await;

    let found = UserRepository::find_by_id(&mut conn, user.id)
        .await
        .unwrap();
    assert_eq!(found.map(|u| u.id), Some(user.id));

    let missing = UserRepository::find_by_id(&mut conn, Uuid::new_v4())
        .await
        .unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn find_all_contains_inserted_rows() {
    let mut conn = test_conn().await;
    let a = seed_user(&mut conn).await;
    let b = seed_user(&mut conn).await;

    let ids: Vec<Uuid> = UserRepository::find_all(&mut conn)
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.id)
        .collect();
    assert!(ids.contains(&a.id) && ids.contains(&b.id));
}

#[tokio::test]
async fn delete_returns_true_then_false() {
    let mut conn = test_conn().await;
    let user = seed_user(&mut conn).await;

    assert!(UserRepository::delete(&mut conn, user.id).await.unwrap());
    assert!(!UserRepository::delete(&mut conn, user.id).await.unwrap());
    assert!(
        UserRepository::find_by_id(&mut conn, user.id)
            .await
            .unwrap()
            .is_none()
    );
}

/// Vérifie que `BaseRepository` est utilisable comme borne générique.
async fn fetch_via_base<R: BaseRepository<User, Uuid>>(
    conn: &mut AsyncPgConnection,
    id: Uuid,
) -> Option<User> {
    R::find_by_id(conn, id).await.unwrap()
}

#[tokio::test]
async fn base_repository_is_usable_as_generic_bound() {
    let mut conn = test_conn().await;
    let user = seed_user(&mut conn).await;
    let found = fetch_via_base::<UserRepository>(&mut conn, user.id).await;
    assert_eq!(found.map(|u| u.id), Some(user.id));
}

// ---------------------------------------------------------------------------
// Exists / Count
// ---------------------------------------------------------------------------

#[tokio::test]
async fn exists_reflects_presence() {
    let mut conn = test_conn().await;
    let user = seed_user(&mut conn).await;

    assert!(UserExtrasRepo::exists(&mut conn, user.id).await.unwrap());
    assert!(
        !UserExtrasRepo::exists(&mut conn, Uuid::new_v4())
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn count_increases_with_inserts() {
    let mut conn = test_conn().await;
    let before = UserExtrasRepo::count(&mut conn).await.unwrap();
    seed_user(&mut conn).await;
    seed_user(&mut conn).await;
    assert_eq!(UserExtrasRepo::count(&mut conn).await.unwrap(), before + 2);
}

// ---------------------------------------------------------------------------
// Paginate
// ---------------------------------------------------------------------------

#[tokio::test]
async fn find_paginated_respects_page_size_and_total() {
    let mut conn = test_conn().await;
    for _ in 0..3 {
        seed_user(&mut conn).await;
    }
    let total = users::table
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    let page = UserPaginateRepo::find_paginated(&mut conn, PaginationParams::new(1, 2))
        .await
        .unwrap();

    assert_eq!(page.total, total);
    assert_eq!(page.data.len(), 2);
    assert_eq!(page.per_page, 2);
    assert_eq!(page.total_pages, (total + 1) / 2);
}

#[tokio::test]
async fn find_paginated_by_filters_and_orders_desc() {
    let mut conn = test_conn().await;
    let owner = seed_user(&mut conn).await;
    let other = seed_user(&mut conn).await;

    let base = Utc::now();
    let mut expected = Vec::new();
    for i in 0..5 {
        expected.push(
            seed_post(&mut conn, owner.id, base + Duration::minutes(i))
                .await
                .id,
        );
    }
    seed_post(&mut conn, other.id, base).await;
    expected.reverse(); // ordre attendu : du plus récent au plus ancien

    let owner_id = owner.id;
    let filter = || -> Filter<posts::table> { Box::new(posts::user_id.eq(owner_id)) };

    let mut seen = Vec::new();
    for page_number in 1..=3 {
        let page = PostRepository::find_paginated_by(
            &mut conn,
            PaginationParams::new(page_number, 2),
            filter,
        )
        .await
        .unwrap();

        assert_eq!(page.total, 5);
        assert_eq!(page.total_pages, 3);
        assert_eq!(page.page, page_number);
        assert!(page.data.iter().all(|p| p.user_id == owner.id));
        seen.extend(page.data.into_iter().map(|p| p.id));
    }

    assert_eq!(
        seen, expected,
        "pages ordonnées par created_at DESC, sans doublon"
    );
}

#[tokio::test]
async fn find_paginated_by_beyond_last_page_is_empty() {
    let mut conn = test_conn().await;
    let owner = seed_user(&mut conn).await;
    seed_post(&mut conn, owner.id, Utc::now()).await;

    let owner_id = owner.id;
    let page = PostRepository::find_paginated_by(&mut conn, PaginationParams::new(5, 10), || {
        Box::new(posts::user_id.eq(owner_id))
    })
    .await
    .unwrap();

    assert!(page.data.is_empty());
    assert_eq!(page.total, 1);
    assert_eq!(page.total_pages, 1);
}

// ---------------------------------------------------------------------------
// SoftDelete
// ---------------------------------------------------------------------------

#[tokio::test]
async fn soft_delete_hides_then_restore_shows() {
    let mut conn = test_conn().await;
    create_soft_items_table(&mut conn).await;

    let id = Uuid::new_v4();
    diesel::insert_into(soft_items::table)
        .values((soft_items::id.eq(id), soft_items::name.eq("item")))
        .execute(&mut conn)
        .await
        .unwrap();

    assert_eq!(SoftItemRepo::find_active(&mut conn).await.unwrap().len(), 1);

    // soft_delete : la ligne reste en base mais n'est plus active
    assert!(SoftItemRepo::soft_delete(&mut conn, id).await.unwrap());
    assert!(
        SoftItemRepo::find_active(&mut conn)
            .await
            .unwrap()
            .is_empty()
    );
    let row = SoftItemRepo::find_by_id(&mut conn, id)
        .await
        .unwrap()
        .unwrap();
    assert!(row.deleted_at.is_some());

    // restore : la ligne redevient active
    assert!(SoftItemRepo::restore(&mut conn, id).await.unwrap());
    assert_eq!(SoftItemRepo::find_active(&mut conn).await.unwrap().len(), 1);

    // id inconnu → false
    assert!(
        !SoftItemRepo::soft_delete(&mut conn, Uuid::new_v4())
            .await
            .unwrap()
    );
}
