//! Teste manual do cartão do Discord, lendo as respostas que o Discord devolve (o app não lê).
//! Uso: com a variável DISCORD_APP_ID definida, `cargo run -p servicos --example discord_teste`.
//! Mostra o cartão por 20 segundos e tira.

use discord_rich_presence::activity::{Activity, ActivityType, StatusDisplayType};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

/// Resume uma resposta do Discord sem mostrar dados pessoais (só comando, evento e erro).
fn resumo(op: u32, v: &serde_json::Value) -> String {
    let cmd = v.get("cmd").and_then(|x| x.as_str()).unwrap_or("-");
    let evt = v.get("evt").and_then(|x| x.as_str()).unwrap_or("-");
    let erro = v.get("data").and_then(|d| d.get("message")).and_then(|x| x.as_str()).unwrap_or("");
    let codigo = v.get("data").and_then(|d| d.get("code")).map(|x| x.to_string()).unwrap_or_default();
    let tipo = v.get("data").and_then(|d| d.get("type")).map(|x| x.to_string()).unwrap_or_default();
    format!("op={op} cmd={cmd} evt={evt} codigo={codigo} tipo={tipo} {erro}")
}

fn main() {
    let Ok(id) = std::env::var("DISCORD_APP_ID") else {
        println!("sem DISCORD_APP_ID");
        return;
    };
    println!("id com {} dígitos, só números: {}", id.len(), id.chars().all(|c| c.is_ascii_digit()));
    let mut c = DiscordIpcClient::new(&id);
    if let Err(e) = c.connect_ipc() {
        println!("não ligou ao canal: {e}");
        return;
    }
    let _ = c.send(serde_json::json!({ "v": 1, "client_id": id }), 0);
    match c.recv() {
        Ok((op, v)) => println!("aperto de mão: {}", resumo(op, &v)),
        Err(e) => {
            println!("aperto de mão sem resposta: {e}");
            return;
        }
    }
    let atividade = Activity::new()
        .activity_type(ActivityType::Listening)
        .status_display_type(StatusDisplayType::Details)
        .details("Teste do Player")
        .state("Artista de teste");
    if let Err(e) = c.set_activity(atividade) {
        println!("não enviou: {e}");
        return;
    }
    match c.recv() {
        Ok((op, v)) => println!("resposta ao cartão: {}", resumo(op, &v)),
        Err(e) => println!("cartão sem resposta: {e}"),
    }
    println!("cartão no ar por 20 s...");
    std::thread::sleep(std::time::Duration::from_secs(20));
    let _ = c.clear_activity();
    let _ = c.close();
    println!("fim");
}
