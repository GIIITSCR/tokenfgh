use std::process::Stdio;
use tokio::process::Command;
use teloxide::prelude::*;
use axum::{routing::get, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

struct AppState {
    is_running: bool,
    iso_path: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let state = Arc::new(Mutex::new(AppState {
        is_running: false,
        iso_path: "ubuntu-server.iso".to_string(),
    }));

    // 1. Скачивание ISO (если нет)
    ensure_iso_exists("https://releases.ubuntu.com/22.04/ubuntu-22.04.3-live-server-amd64.iso").await;

    // 2. Запуск Cloudflare Tunnel (как дочерний процесс)
    spawn_cloudflare_tunnel();

    // 3. Запуск Веб-сервера (Axum)
    let app_state = state.clone();
    let app = Router::new()
        .route("/", get(|| async { "QEMU Control Panel: Online" }))
        .route("/start", get(move || start_vm_handler(app_state.clone())));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    
    // 4. Запуск Telegram Бота
    let bot_handle = tokio::spawn(run_telegram_bot(state.clone()));

    println!("🚀 Сервис запущен на http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

// --- Логика QEMU ---

async fn start_vm(iso: &str) {
    let mut child = Command::new("qemu-system-x86_64")
        .args([
            "-m", "2048",
            "-cdrom", iso,
            "-drive", "file=storage.qcow2,format=qcow2",
            "-nographic", // Для серверной версии без GUI
            "-vnc", ":1", // Доступ по VNC
            "-monitor", "stdio"
        ])
        .spawn()
        .expect("Не удалось запустить QEMU");
    
    child.wait().await.ok();
}

// --- Интеграция с Cloudflare ---

fn spawn_cloudflare_tunnel() {
    let token = std::env::var("CF_TUNNEL_TOKEN").expect("Нужен CF_TUNNEL_TOKEN");
    tokio::spawn(async move {
        Command::new("cloudflared")
            .args(["tunnel", "--no-autoupdate", "run", "--token", &token])
            .spawn()
            .expect("Ошибка запуска cloudflared");
    });
}

// --- Telegram Бот ---

async fn run_telegram_bot(state: Arc<Mutex<AppState>>) {
    let bot = Bot::from_env();
    
    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let state = state.clone();
        async move {
            match msg.text() {
                Some("/start") => {
                    let mut s = state.lock().await;
                    if !s.is_running {
                        let iso = s.iso_path.clone();
                        tokio::spawn(async move { start_vm(&iso).await });
                        s.is_running = true;
                        bot.send_message(msg.chat.id, "✅ VM запущена!").await?;
                    }
                }
                _ => { bot.send_message(msg.chat.id, "Команды: /start, /status").await?; }
            }
            Ok(())
        }
    }).await;
}

async fn ensure_iso_exists(url: &str) {
    if !std::path::Path::new("ubuntu-server.iso").exists() {
        println!("📥 Загрузка Ubuntu Server...");
        // Тут логика reqwest для записи файла
    }
}

async fn start_vm_handler(state: Arc<Mutex<AppState>>) -> &'static str {
    // Аналогичная логика старта для веб-интерфейса
    "Запрос на запуск получен"
}
