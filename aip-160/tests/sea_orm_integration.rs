#![cfg(feature = "sea-orm")]

use aip_160::{ToSeaOrmCondition, parse_filter};
use sea_orm::{
    Database, DatabaseConnection, DbBackend, DbErr, QueryOrder, QuerySelect, QueryTrait, Set,
    entity::prelude::*,
};

// Define test entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: i32,
    pub active: bool,
    pub score: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// Define UUID test entity for PostgreSQL-specific tests
mod uuid_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "documents")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub title: String,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// Define entity with enum column for PostgreSQL-specific tests
mod enum_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status_type")]
    pub enum Status {
        #[sea_orm(string_value = "pending")]
        Pending,
        #[sea_orm(string_value = "active")]
        Active,
        #[sea_orm(string_value = "completed")]
        Completed,
    }

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tasks")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub status: Status,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// Helper function to setup test database
async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;

    // Create table
    let create_table_sql = r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            age INTEGER NOT NULL,
            active BOOLEAN NOT NULL,
            score REAL NOT NULL
        )
    "#;

    db.execute_unprepared(create_table_sql).await?;

    Ok(db)
}

// Helper function to seed test data
async fn seed_data(db: &DatabaseConnection) -> Result<(), DbErr> {
    let users = vec![
        (1, "Alice", "alice@example.com", 25, true, 95.5),
        (2, "Bob", "bob@example.com", 30, true, 87.3),
        (3, "Charlie", "charlie@test.com", 22, false, 76.8),
        (4, "David", "david@example.com", 35, true, 91.2),
        (5, "Eve", "eve@test.com", 28, false, 82.4),
        (6, "Frank", "frank@example.com", 42, true, 88.9),
    ];

    for (id, name, email, age, active, score) in users {
        let user = ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            email: Set(email.to_string()),
            age: Set(age),
            active: Set(active),
            score: Set(score),
        };
        Entity::insert(user).exec(db).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_simple_equality_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("name = \"Alice\"")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[0].age, 25);

    Ok(())
}

#[tokio::test]
async fn test_number_comparison() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("age > 30")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 2); // David (35) and Frank (42)
    assert!(users.iter().all(|u| u.age > 30));

    Ok(())
}

#[tokio::test]
async fn test_float_comparison() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("score >= 90.0")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 2); // Alice (95.5) and David (91.2)
    assert!(users.iter().all(|u| u.score >= 90.0));

    Ok(())
}

#[tokio::test]
async fn test_boolean_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("active = true")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 4); // Alice, Bob, David, Frank
    assert!(users.iter().all(|u| u.active));

    Ok(())
}

#[tokio::test]
async fn test_and_expression() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("age > 25 AND active = true")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 3); // Bob, David, Frank
    assert!(users.iter().all(|u| u.age > 25 && u.active));

    Ok(())
}

#[tokio::test]
async fn test_or_expression() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("name = \"Alice\" OR name = \"Bob\"")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 2);
    assert!(users.iter().any(|u| u.name == "Alice"));
    assert!(users.iter().any(|u| u.name == "Bob"));

    Ok(())
}

#[tokio::test]
async fn test_not_expression() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("NOT active = false")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 4); // All active users
    assert!(users.iter().all(|u| u.active));

    Ok(())
}

#[tokio::test]
async fn test_has_operator_like() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("email : \"@example.com\"")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 4); // Alice, Bob, David, Frank
    assert!(users.iter().all(|u| u.email.contains("@example.com")));

    Ok(())
}

#[tokio::test]
async fn test_complex_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    // Find active users older than 25 with score >= 85
    let filter = parse_filter("active = true AND age > 25 AND score >= 85.0")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 3); // Bob, David, Frank
    assert!(
        users
            .iter()
            .all(|u| u.active && u.age > 25 && u.score >= 85.0)
    );

    Ok(())
}

#[tokio::test]
async fn test_grouped_expression() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    // (name = "Alice" OR name = "Bob") AND age >= 25
    let filter = parse_filter("(name = \"Alice\" OR name = \"Bob\") AND age >= 25")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 2); // Alice (25) and Bob (30)
    assert!(
        users
            .iter()
            .all(|u| (u.name == "Alice" || u.name == "Bob") && u.age >= 25)
    );

    Ok(())
}

#[tokio::test]
async fn test_less_than_or_equal() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("age <= 28")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 3); // Alice (25), Charlie (22), Eve (28)
    assert!(users.iter().all(|u| u.age <= 28));

    Ok(())
}

#[tokio::test]
async fn test_not_equal() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("name != \"Alice\"")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find().filter(condition).all(&db).await?;

    assert_eq!(users.len(), 5);
    assert!(users.iter().all(|u| u.name != "Alice"));

    Ok(())
}

#[tokio::test]
async fn test_ordering_with_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("active = true")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find()
        .filter(condition)
        .order_by_asc(Column::Age)
        .all(&db)
        .await?;

    assert_eq!(users.len(), 4);
    // Check ordering
    assert_eq!(users[0].name, "Alice"); // age 25
    assert_eq!(users[1].name, "Bob"); // age 30
    assert_eq!(users[2].name, "David"); // age 35
    assert_eq!(users[3].name, "Frank"); // age 42

    Ok(())
}

#[tokio::test]
async fn test_pagination_with_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("active = true")?;
    let condition = filter.to_condition::<Entity>()?;

    let users = Entity::find()
        .filter(condition)
        .order_by_asc(Column::Id)
        .limit(2)
        .offset(1)
        .all(&db)
        .await?;

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Bob"); // id 2
    assert_eq!(users[1].name, "David"); // id 4

    Ok(())
}

#[tokio::test]
async fn test_count_with_filter() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("score > 85.0")?;
    let condition = filter.to_condition::<Entity>()?;

    let count = Entity::find().filter(condition).count(&db).await?;

    assert_eq!(count, 4); // Alice, Bob, David, Frank

    Ok(())
}

// Test that UUID columns generate proper CAST in SQL
#[test]
fn test_uuid_cast_in_sql() -> Result<(), Box<dyn std::error::Error>> {
    // Test equality
    let filter = parse_filter("id = \"550e8400-e29b-41d4-a716-446655440000\"")?;
    let condition = filter.to_condition::<uuid_entity::Entity>()?;
    let query = uuid_entity::Entity::find().filter(condition);
    let sql = query.build(DbBackend::Postgres).to_string();

    assert!(
        sql.contains("CAST"),
        "SQL should contain CAST for UUID: {}",
        sql
    );
    assert!(
        sql.contains("uuid"),
        "SQL should cast to uuid type: {}",
        sql
    );

    Ok(())
}

// Test that UUID columns with comparison operators also generate CAST
#[test]
fn test_uuid_cast_comparison_operators() -> Result<(), Box<dyn std::error::Error>> {
    // Test greater than
    let filter = parse_filter("id > \"550e8400-e29b-41d4-a716-446655440000\"")?;
    let condition = filter.to_condition::<uuid_entity::Entity>()?;
    let query = uuid_entity::Entity::find().filter(condition);
    let sql = query.build(DbBackend::Postgres).to_string();

    assert!(
        sql.contains("CAST"),
        "SQL should contain CAST for UUID comparison: {}",
        sql
    );
    assert!(
        sql.contains("uuid"),
        "SQL should cast to uuid type: {}",
        sql
    );

    // Test less than
    let filter2 = parse_filter("id < \"550e8400-e29b-41d4-a716-446655440000\"")?;
    let condition2 = filter2.to_condition::<uuid_entity::Entity>()?;
    let query2 = uuid_entity::Entity::find().filter(condition2);
    let sql2 = query2.build(DbBackend::Postgres).to_string();

    assert!(
        sql2.contains("CAST"),
        "SQL should contain CAST for UUID lt comparison: {}",
        sql2
    );

    Ok(())
}

// Test that timestamp columns generate proper CAST in SQL
#[test]
fn test_timestamp_cast_in_sql() -> Result<(), Box<dyn std::error::Error>> {
    let filter = parse_filter("created_at > \"2024-01-01T00:00:00Z\"")?;
    let condition = filter.to_condition::<uuid_entity::Entity>()?;
    let query = uuid_entity::Entity::find().filter(condition);
    let sql = query.build(DbBackend::Postgres).to_string();

    assert!(
        sql.contains("CAST"),
        "SQL should contain CAST for timestamp: {}",
        sql
    );
    assert!(
        sql.contains("timestamptz"),
        "SQL should cast to timestamptz type: {}",
        sql
    );

    Ok(())
}

// Test that enum columns generate proper CAST in SQL
#[test]
fn test_enum_cast_in_sql() -> Result<(), Box<dyn std::error::Error>> {
    let filter = parse_filter("status = \"active\"")?;
    let condition = filter.to_condition::<enum_entity::Entity>()?;
    let query = enum_entity::Entity::find().filter(condition);
    let sql = query.build(DbBackend::Postgres).to_string();

    assert!(
        sql.contains("CAST"),
        "SQL should contain CAST for enum: {}",
        sql
    );
    assert!(
        sql.contains("status_type"),
        "SQL should cast to enum type name: {}",
        sql
    );

    Ok(())
}

// AIP-160 gives OR a higher precedence than AND:
// `a AND b OR c` means `a AND (b OR c)`.
#[tokio::test]
async fn test_or_binds_tighter_than_and() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("name = \"Charlie\" AND age > 40 OR active = false")?;
    let users = Entity::find()
        .filter(filter.to_condition::<Entity>()?)
        .all(&db)
        .await?;

    // With AND first, Eve would also match.
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Charlie");

    Ok(())
}

#[tokio::test]
async fn test_sequence_means_and() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("active = true age > 30")?;
    let users = Entity::find()
        .filter(filter.to_condition::<Entity>()?)
        .all(&db)
        .await?;

    assert_eq!(users.len(), 2); // David (35) and Frank (42)
    Ok(())
}

#[tokio::test]
async fn test_empty_filter_matches_all() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let filter = parse_filter("")?;
    let count = Entity::find()
        .filter(filter.to_condition::<Entity>()?)
        .count(&db)
        .await?;

    assert_eq!(count, 6);
    Ok(())
}

#[tokio::test]
async fn test_wildcard_strings() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;
    seed_data(&db).await?;

    let prefix = parse_filter("name = \"A*\"")?;
    let users = Entity::find()
        .filter(prefix.to_condition::<Entity>()?)
        .all(&db)
        .await?;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Alice");

    let suffix = parse_filter("email = \"*@test.com\"")?;
    let count = Entity::find()
        .filter(suffix.to_condition::<Entity>()?)
        .count(&db)
        .await?;
    assert_eq!(count, 2); // Charlie and Eve

    // LIKE metacharacters in the value are matched literally.
    let literal = parse_filter("email : \"_\"")?;
    let count = Entity::find()
        .filter(literal.to_condition::<Entity>()?)
        .count(&db)
        .await?;
    assert_eq!(count, 0);

    Ok(())
}

#[test]
fn test_unquoted_enum_value_is_cast() -> Result<(), Box<dyn std::error::Error>> {
    let filter = parse_filter("status = active")?;
    let condition = filter.to_condition::<enum_entity::Entity>()?;
    let sql = enum_entity::Entity::find()
        .filter(condition)
        .build(DbBackend::Postgres)
        .to_string();

    assert!(sql.contains("CAST('active' AS status_type)"), "{sql}");
    Ok(())
}

#[test]
fn test_rejects_values_that_do_not_fit_the_column() {
    use aip_160::FilterError;

    let invalid = |input: &str| parse_filter(input).unwrap().to_condition::<Entity>();
    assert!(matches!(
        invalid("age > abc"),
        Err(FilterError::InvalidValue { .. })
    ));
    assert!(matches!(
        invalid("active = yes"),
        Err(FilterError::InvalidValue { .. })
    ));
    assert!(matches!(
        invalid("score < \"1\""),
        Err(FilterError::InvalidValue { .. })
    ));
    assert!(matches!(
        invalid("missing = 1"),
        Err(FilterError::InvalidField(_))
    ));
    assert!(matches!(invalid("Alice"), Err(FilterError::Unsupported(_))));
    assert!(matches!(
        invalid("name.first = x"),
        Err(FilterError::Unsupported(_))
    ));
    assert!(matches!(invalid("age:1"), Err(FilterError::Unsupported(_))));
}
