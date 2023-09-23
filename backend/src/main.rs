use axum::{
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::{
    types::chrono::NaiveDateTime,
    Executor, FromRow, PgPool, Postgres, QueryBuilder,
};

use tower_http::cors::CorsLayer;

use rust_decimal;

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:7622043385@localhost:5432/postgres"
    )]
    pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    pool.execute(include_str!("../schema.sql"))
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin("http://127.0.0.1:5173".parse::<HeaderValue>().unwrap());

    let router = Router::new()
        .route("/items", post(create_item))
        .route("/items", get(get_items))
        .route("/items/:id", get(get_item))
        .route("/items/:id", put(update_item))
        .route("/items/:id", delete(delete_item))
        .route("/balance", get(get_balance))
        .route("/categories", get(get_categories))
        .layer(cors_layer)
        .with_state(pool);

    Ok(router.into())
}

async fn create_item(
    State(pool): State<PgPool>,
    Json(request_data): Json<RequestData>,
) -> impl IntoResponse {
    // process category, check if exists or create new
    let new_category = request_data.category;
    let new_item = request_data.item;
    let cat_result: Option<Category> =
        sqlx::query_as(&Category::select(new_category.category.trim().to_string()))
            .fetch_one(&pool)
            .await
            .map_err(|_| {})
            .ok();

    let query_category = cat_result;
    let category_id;
    match query_category {
        Some(c) => {
            println!("{:?}", c);
            category_id = c.category_id
        }
        None => {
            println!("Category {} does not yet exists", new_category.category);
            let result: (i32,) = sqlx::query_as(&new_category.insert(&None))
                .fetch_one(&pool)
                .await
                .unwrap();
            category_id = result.0;
        }
    }
    // push budget item to db

    let result = sqlx::query(&new_item.insert(&Some(category_id)))
        .execute(&pool)
        .await;

    match result {
        Ok(_) => (StatusCode::OK, "Article created".to_string()),
        Err(e) => internal_error("Error creating item", e),
    }
}

async fn get_item(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<Json<Item>, (StatusCode, String)> {
    let item: Item = sqlx::query_as(&Item::select(id))
        .fetch_one(&pool)
        .await
        .map_err(|e| internal_error("Item not found", e))?;
    Ok(Json(item))
}

async fn update_item(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    Json(request_data): Json<RequestData>,
) -> Result<Json<Item>, (StatusCode, String)> {
    let updated_item = request_data.item;

    todo!()
}

async fn delete_item(Path(id): Path<i32>, State(pool): State<PgPool>) -> impl IntoResponse {
    let result = sqlx::query(&Item::delete(id))
        .fetch_one(&pool)
        .await;
    match result {
        Ok(_) => (StatusCode::OK, format!("Item deleted")),
        Err(e) => internal_error("Error deleting item", e),
    }
}

async fn get_items(State(pool): State<PgPool>) -> Result<Json<Vec<Item>>, (StatusCode, String)> {
    println!("Getting items");
    let items: Vec<Item> = sqlx::query_as("SELECT * FROM budget_items JOIN category ON category.category_id=budget_items.category_id JOIN item_types ON item_types.type_id=budget_items.type_id;")
        .fetch_all(&pool)
        .await
        .unwrap();
    println!("{:?}", items);
    Ok(Json(items))
}

async fn get_balance(State(pool): State<PgPool>) -> Result<Json<String>, (StatusCode, String)> {
    let amounts: Vec<Amount> =
        sqlx::query_as("SELECT *  FROM  (SELECT budget_items.type_id, SUM(amount) FROM budget_items GROUP BY budget_items.type_id) AS sums JOIN item_types ON item_types.type_id=sums.type_id;")
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                internal_error("Error getting balance", e)
            })?;

    println!("{:?}", amounts);
    let mut result = rust_decimal::Decimal::new(0, 1);
    for amount in amounts {
        if amount.item_type == "expense" {
            result -= amount.sum;
        } else if amount.item_type == "income" {
            result += amount.sum
        } else {
            ();
        }
    }
    Ok(Json(result.to_string()))
}
async fn get_categories(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Category>>, (StatusCode, String)> {
    let cagories: Vec<Category> = sqlx::query_as("SELECT * FROM category;")
        .fetch_all(&pool)
        .await
        .map_err(|e| internal_error("Error retrieving cateogories", e))?;
    println!("{:?}", cagories);
    Ok(Json(cagories))
}

trait SQLStatements<T> {
    fn insert(&self, opt_id: &Option<i32>) -> String;
    fn select(key: T) -> String;
    fn delete(key: T) -> String;

}

impl SQLStatements<i32> for Item {
    fn insert(&self, opt_id: &Option<i32>) -> String {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO budget_items (name, amount, description, type_id, category_id)",
        );
        query_builder.push_values([self], |mut b, item| {
            b.push_bind(item.name.clone())
                .push_bind(item.amount.clone())
                .push_bind(item.description.clone())
                .push_bind(item.type_id.clone())
                .push_bind(opt_id.unwrap());
        });

        query_builder.into_sql()
    }
    fn select(key: i32) -> String {
        format!("SELECT * FROM budget_items JOIN category ON category.category_id = budget_items.category_id JOIN item_types ON item_types.type_id=budget_items.type_id WHERE budget_items.id = {};", key)
    }

    fn delete(key: i32) -> String {
        format!("DELETE FROM budget_items WHERE id={}",key)
    }
}
impl SQLStatements<String> for Category {
    fn select(key: String) -> String {
        format!("SELECT * FROM category WHERE category={}", key)
    }
    fn insert(&self, opt_id: &Option<i32>) -> String {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO category(category) VALUES ($1) RETURNING category_id");

        query_builder.push_values([self], |mut b, category| {
            b.push_bind(category.category.trim());
        });
        query_builder.into_sql()
    }

    fn delete(key: String) -> String {
        format!("DELETE FROM category WHERE category_id = {}", key)
    }
}

fn internal_error(message: &str, e: sqlx::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("{}: {}", message, e),
    )
}

#[derive(Deserialize, Serialize, FromRow, Debug)]
struct Item {
    #[serde(skip_deserializing)]
    id: i32,
    name: String,
    amount: rust_decimal::Decimal,
    description: String,
    type_id: i32,
    #[serde(skip_deserializing)]
    category_id: i32,
    #[serde(skip_deserializing)]
    date: NaiveDateTime,
    #[serde(skip_serializing_if = "String::is_empty", skip_deserializing)]
    category: String,
    #[serde(skip_serializing_if = "String::is_empty", skip_deserializing)]
    item_type: String,
}
#[derive(Deserialize, Serialize, Debug, FromRow)]
struct Category {
    category: String,
    #[serde(skip_deserializing)]
    category_id: i32,
}

#[derive(Deserialize)]
struct RequestData {
    item: Item,
    category: Category,
}

#[derive(FromRow, Serialize, Debug)]
struct Amount {
    sum: rust_decimal::Decimal,
    type_id: i32,
    item_type: String,
}
