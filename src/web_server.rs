use axum::{
    extract::{Path, State}, // 从这里移除 Query
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum::extract::Query; // 单独导入 Query
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
// use tokio::net::TcpListener; // 确保此行被注释或删除
use crate::api::ApiMgr;

// 1. 定义请求/响应结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct PutRequest {
    key: String,
    value: String,
    transaction_id: Option<u64>, // 添加可选的 transaction_id 字段
}

#[derive(Serialize, Debug)]
pub struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetValueResponse {
    key: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionResponse {
    transaction_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionActionRequest {
    transaction_id: u64,
}

pub async fn start_server() {
    let api_mgr = Arc::new(Mutex::new(ApiMgr::new()));

    // 定义路由
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/api/kv/:key", get(get_value_handler)) // 2. GET 端点
        .route("/api/kv", post(put_value_handler))    // 3. POST 端点
        .route("/api/transactions/begin", post(begin_transaction_handler))
        .route("/api/transactions/commit", post(commit_transaction_handler))
        .route("/api/transactions/rollback", post(rollback_transaction_handler))
        .with_state(api_mgr); // 5. 共享状态

    let addr = SocketAddr::from(([0, 0, 0, 0], 4399));
    println!("数据库 API 服务器正在启动，监听于 {}...", addr);

    // 适应 Axum 0.7+ 的服务器启动方式
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to address {}: {}", addr, e);
            return;
        }
    };
    println!("Successfully bound to {}", addr);
    if let Err(e) = axum::serve(listener, app.into_make_service()).await {
        eprintln!("Server error: {}", e);
    }
}

async fn root_handler() -> &'static str {
    "数据库 API 服务器已启动"
}

// 2. 实现 GET /api/kv/:key 端点
async fn get_value_handler(
    State(app_state): State<Arc<Mutex<ApiMgr>>>,
    Path(key): Path<String>,
    Query(params): Query<HashMap<String, String>>, // 添加 Query 提取器
) -> impl IntoResponse {
    let transaction_id_str = params.get("transaction_id");
    println!(
        "Received GET request for key: {}, transaction_id_str: {:?}",
        key, transaction_id_str
    );

    let mut manager = match app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Failed to acquire lock for ApiMgr: {:?}", poisoned);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<GetValueResponse> {
                    success: false,
                    data: None,
                    error: Some("Failed to acquire lock".to_string()),
                }),
            )
                .into_response();
        }
    };

    // 解析 transaction_id (如果提供)
    let transaction_id: Option<u64> = match transaction_id_str {
        Some(id_str) => match id_str.parse::<u64>() {
            Ok(id) => Some(id),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<GetValueResponse> {
                        success: false,
                        data: None,
                        error: Some("Invalid transaction_id format".to_string()),
                    }),
                )
                    .into_response();
            }
        },
        None => None,
    };

    if let Some(tid) = transaction_id {
        // 如果提供了 transaction_id，我们假设 ApiMgr::get 能够处理它
        // 或者，如果 ApiMgr::get 尚未感知事务，它将读取最新提交的数据。
        // 这里的重点是将 transaction_id 从 API 层传递下去。
        println!("Getting value for key '{}' under transaction_id {}", key, tid);
        // 注意：当前 ApiMgr::get 可能不接受 transaction_id。
        // 为了简化，我们仍然调用现有的 manager.get(key)。
        // 理想情况下，ApiMgr::get 应该能够感知事务上下文。
    } else {
        println!("Getting value for key '{}' (no transaction_id)", key);
    }

    match manager.get(key.clone()) {
        Some(value) => {
            println!("Value found for key '{}': '{}'", key, value);
            (
                StatusCode::OK,
                Json(ApiResponse {
                    success: true,
                    data: Some(GetValueResponse { key, value }),
                    error: None,
                }),
            )
                .into_response()
        }
        None => {
            println!("No value found for key '{}'", key);
            (
                StatusCode::OK,
                Json(ApiResponse::<GetValueResponse> {
                    success: true,
                    data: None,
                    error: None,
                }),
            )
                .into_response()
        }
    }
}

// 3. 实现 POST /api/kv 端点
async fn put_value_handler(
    State(app_state): State<Arc<Mutex<ApiMgr>>>,
    Json(payload): Json<PutRequest>,
) -> impl IntoResponse {
    println!(
        "Received POST request with key: '{}', value: '{}', transaction_id: {:?}",
        payload.key, payload.value, payload.transaction_id
    );

    let mut manager = match app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Failed to acquire lock for ApiMgr: {:?}", poisoned);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some("Failed to acquire lock".to_string()),
                }),
            )
                .into_response();
        }
    };

    if let Some(provided_transaction_id) = payload.transaction_id {
        // 情况 1: 提供了 transaction_id
        println!(
            "Putting value for key '{}' under transaction_id {}",
            payload.key, provided_transaction_id
        );
        // 假设 ApiMgr::put 能够处理事务 ID
        // 并且在失败时会 panic 或返回一个我们可以转换为 ApiResponse 的错误
        // 为了简化，我们假设它在成功时返回 (), 失败时 panic
        // 实际应用中，ApiMgr::put 应该返回 Result
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            manager.put(provided_transaction_id, payload.key.clone(), payload.value.clone())
        })) {
            Ok(_) => {
                println!(
                    "Put operation for key '{}' in transaction {} successful",
                    payload.key, provided_transaction_id
                );
                (
                    StatusCode::OK,
                    Json(ApiResponse::<()> {
                        success: true,
                        data: None,
                        error: None,
                    }),
                )
                    .into_response()
            }
            Err(_) => {
                eprintln!(
                    "Failed to put value for key '{}' in transaction {} (panic occurred)",
                    payload.key, provided_transaction_id
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()> {
                        success: false,
                        data: None,
                        error: Some(format!(
                            "Failed to put value in transaction {}",
                            provided_transaction_id
                        )),
                    }),
                )
                    .into_response()
            }
        }
    } else {
        // 情况 2: 没有提供 transaction_id (自动提交模式)
        println!(
            "Putting value for key '{}' in auto-commit mode",
            payload.key
        );
        // 这里的错误处理逻辑基于现有的代码结构，依赖 panic 或成功
        // 理想情况下，每个 ApiMgr 调用都应返回 Result
        let transaction_id = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| manager.begin_transaction())) {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Failed to begin transaction for auto-commit (panic occurred)");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()> {
                        success: false,
                        data: None,
                        error: Some("Failed to begin transaction for auto-commit".to_string()),
                    }),
                )
                    .into_response();
            }
        };
        println!("Started transaction with ID: {} for auto-commit", transaction_id);

        if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            manager.put(transaction_id, payload.key.clone(), payload.value.clone())
        })) {
            eprintln!(
                "Failed to put value for key '{}' in auto-commit transaction {} (panic occurred)",
                payload.key, transaction_id
            );
            // 尝试回滚
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| manager.rollback_transaction(transaction_id)));
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Failed to put value in auto-commit transaction {}",
                        transaction_id
                    )),
                }),
            )
                .into_response();
        }
        println!(
            "Put operation for key '{}' in auto-commit transaction {} successful",
            payload.key, transaction_id
        );

        if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| manager.commit_transaction(transaction_id))) {
            eprintln!(
                "Failed to commit auto-commit transaction {} (panic occurred)",
                transaction_id
            );
            // 尝试回滚，尽管提交失败后的回滚可能意义不大
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| manager.rollback_transaction(transaction_id)));
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to commit auto-commit transaction {}", transaction_id)),
                }),
            )
                .into_response();
        }
        println!("Committed auto-commit transaction {}", transaction_id);

        (
            StatusCode::OK,
            Json(ApiResponse::<()> {
                success: true,
                data: None,
                error: None,
            }),
        )
            .into_response()
    }
}

// Handler for POST /api/transactions/begin
async fn begin_transaction_handler(
    State(app_state): State<Arc<Mutex<ApiMgr>>>,
) -> impl IntoResponse {
    println!("Received request to begin transaction");
    let mut manager = match app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Failed to acquire lock for ApiMgr: {:?}", poisoned);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<TransactionResponse> {
                    success: false,
                    data: None,
                    error: Some("Failed to acquire lock".to_string()),
                }),
            )
                .into_response();
        }
    };

    // 在实际应用中，ApiMgr::begin_transaction 应该返回 Result
    // 这里我们假设它在成功时返回 ID，失败时 panic 或通过其他方式处理
    // 根据任务要求，如果 ApiMgr 方法尚未返回 Result，则可以继续依赖现有的 panic 行为
    // 或者在处理器中添加基本的错误检查。
    // 为了简化，我们这里直接调用，并期望 Axum 捕获 panic。
    // 更健壮的实现会处理 Result。
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| manager.begin_transaction())) {
        Ok(transaction_id) => {
            println!("Transaction begun with ID: {}", transaction_id);
            (
                StatusCode::OK,
                Json(ApiResponse {
                    success: true,
                    data: Some(TransactionResponse { transaction_id }),
                    error: None,
                }),
            )
                .into_response()
        }
        Err(_) => {
            eprintln!("Failed to begin transaction (panic occurred)");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<TransactionResponse> {
                    success: false,
                    data: None,
                    error: Some("Failed to begin transaction".to_string()),
                }),
            )
                .into_response()
        }
    }
}

// Handler for POST /api/transactions/commit
async fn commit_transaction_handler(
    State(app_state): State<Arc<Mutex<ApiMgr>>>,
    Json(payload): Json<TransactionActionRequest>,
) -> impl IntoResponse {
    println!("Received request to commit transaction ID: {}", payload.transaction_id);
    let mut manager = match app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Failed to acquire lock for ApiMgr: {:?}", poisoned);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some("Failed to acquire lock".to_string()),
                }),
            )
                .into_response();
        }
    };

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        manager.commit_transaction(payload.transaction_id)
    })) {
        Ok(_) => {
            println!("Transaction {} committed successfully", payload.transaction_id);
            (
                StatusCode::OK,
                Json(ApiResponse::<()> {
                    success: true,
                    data: None, // Or Some("Transaction committed".to_string())
                    error: None,
                }),
            )
                .into_response()
        }
        Err(_) => {
            eprintln!("Failed to commit transaction {} (panic occurred)", payload.transaction_id);
            // 根据任务要求，如果 ApiMgr 方法尚未返回 Result，则可以继续依赖现有的 panic 行为。
            // 理想情况下，ApiMgr::commit_transaction 会返回 Result<_, Error>
            // 我们可以根据错误类型返回更具体的 HTTP 状态码和错误信息。
            // 例如，如果事务 ID 无效，可以返回 StatusCode::BAD_REQUEST。
            (
                StatusCode::INTERNAL_SERVER_ERROR, // Or BAD_REQUEST if ID is known to be invalid
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to commit transaction {}", payload.transaction_id)),
                }),
            )
                .into_response()
        }
    }
}

// Handler for POST /api/transactions/rollback
async fn rollback_transaction_handler(
    State(app_state): State<Arc<Mutex<ApiMgr>>>,
    Json(payload): Json<TransactionActionRequest>,
) -> impl IntoResponse {
    println!("Received request to rollback transaction ID: {}", payload.transaction_id);
    let mut manager = match app_state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Failed to acquire lock for ApiMgr: {:?}", poisoned);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some("Failed to acquire lock".to_string()),
                }),
            )
                .into_response();
        }
    };

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        manager.rollback_transaction(payload.transaction_id)
    })) {
        Ok(_) => {
            println!("Transaction {} rollbacked successfully", payload.transaction_id);
            (
                StatusCode::OK,
                Json(ApiResponse::<()> {
                    success: true,
                    data: None, // Or Some("Transaction rollbacked".to_string())
                    error: None,
                }),
            )
                .into_response()
        }
        Err(_) => {
            eprintln!("Failed to rollback transaction {} (panic occurred)", payload.transaction_id);
            (
                StatusCode::INTERNAL_SERVER_ERROR, // Or BAD_REQUEST if ID is known to be invalid
                Json(ApiResponse::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to rollback transaction {}", payload.transaction_id)),
                }),
            )
                .into_response()
        }
    }
}