use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{NaiveDateTime, Utc};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // Load environment variables from `.env` file
    dotenv().ok();

    // Get the database URL from environment variables
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create a connection pool for PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool");

    // Wrap the connection pool in an Arc for sharing across threads
    let shared_pool = Arc::new(pool);

    // Define routes for the application
    let app = Router::new()
        .route("/products", get(get_products))
        .route("/customers", get(get_customers))
        .route("/orders", get(get_orders).post(add_order))
        .with_state(shared_pool);

    // Define the server address
    let addr = "127.0.0.1:3000".parse().unwrap();
    println!("Server is running on http://{}", addr);

    // Start the Axum server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Struct representing the `Product` table
#[derive(Debug, Serialize, FromRow)]
struct Product {
    id: Uuid,
    name: String,
}

// Struct representing the `Customer` table
#[derive(Debug, Serialize, FromRow)]
struct Customer {
    id: Uuid,
    name: String,
}

// Struct representing the `Order` table
#[derive(Debug, Serialize, FromRow)]
struct Order {
    id: Uuid,
    customer_id: Uuid,
    product_id: Uuid,
    quantity: i32,
    order_date: NaiveDateTime,
}

// Struct for new order input (used in POST `/orders`)
#[derive(Debug, Deserialize)]
struct NewOrder {
    customer_id: Uuid,
    product_id: Uuid,
    quantity: i32,
}

// GET `/products`: Fetch all products
async fn get_products(State(pool): State<Arc<PgPool>>) -> Result<Json<Vec<Product>>, StatusCode> {
    let products = sqlx::query_as::<_, Product>("SELECT id, name FROM product")
        .fetch_all(&*pool)
        .await
        .map_err(|err| {
            eprintln!("Failed to fetch products: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(products))
}

// GET `/customers`: Fetch all customers
async fn get_customers(State(pool): State<Arc<PgPool>>) -> Result<Json<Vec<Customer>>, StatusCode> {
    let customers = sqlx::query_as::<_, Customer>("SELECT id, name FROM customer")
        .fetch_all(&*pool)
        .await
        .map_err(|err| {
            eprintln!("Failed to fetch customers: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(customers))
}

// GET `/orders`: Fetch all orders
async fn get_orders(State(pool): State<Arc<PgPool>>) -> Result<Json<Vec<Order>>, StatusCode> {
    let orders = sqlx::query_as::<_, Order>(
        "SELECT id, customer_id, product_id, quantity, order_date FROM \"order\"",
    )
    .fetch_all(&*pool)
    .await
    .map_err(|err| {
        eprintln!("Failed to fetch orders: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(orders))
}

// POST `/orders`: Add a new order
async fn add_order(
    State(pool): State<Arc<PgPool>>,
    Json(payload): Json<NewOrder>,
) -> Result<Json<Order>, StatusCode> {
    let new_order = sqlx::query_as::<_, Order>(
        "INSERT INTO \"order\" (customer_id, product_id, quantity, order_date)
        VALUES ($1, $2, $3, $4)
        RETURNING id, customer_id, product_id, quantity, order_date",
    )
    .bind(payload.customer_id)
    .bind(payload.product_id)
    .bind(payload.quantity)
    .bind(Utc::now().naive_utc())
    .fetch_one(&*pool)
    .await
    .map_err(|err| {
        eprintln!("Failed to add order: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(new_order))
}
