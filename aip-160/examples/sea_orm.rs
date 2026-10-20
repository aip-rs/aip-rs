use aip_160::{ToSeaOrmCondition, parse_filter};
use sea_orm::entity::prelude::*;

// Define the User entity as usual
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: i32,
    pub active: bool,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[allow(dead_code)]
async fn find_users(db: &DatabaseConnection, filter_str: &str) -> Result<Vec<Model>, DbErr> {
    let filter = parse_filter(filter_str).map_err(|e| DbErr::Custom(e.to_string()))?;

    let condition = filter
        .to_condition::<Entity>()
        .map_err(|e| DbErr::Custom(e.to_string()))?;

    Entity::find().filter(condition).all(db).await
}

fn main() {
    let examples = vec![
        "id = 123",
        "name = \"Alice\"",
        "email : \"@example.com\"",
        "age > 18 AND active = true",
        "created_at > \"2024-01-01\"",
        "(name : \"Smith\" OR email : \"@company.com\") AND age >= 25",
        "NOT active = false",
    ];

    println!("Testing filters:\n");

    for filter_str in examples {
        print!("  {:<70} ", filter_str);

        match parse_filter(filter_str) {
            Ok(filter) => match filter.to_condition::<Entity>() {
                Ok(_) => println!("✓"),
                Err(e) => println!("✗ {}", e),
            },
            Err(e) => println!("✗ {}", e),
        }
    }
}
