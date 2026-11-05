// Hermetic protocol fixture. Never executes commands or contacts a model.
use std::io::{self, BufRead, Write};
fn emit(s: &str) { println!("{s}"); io::stdout().flush().unwrap(); }
fn main() {
    for line in io::stdin().lock().lines() {
        let s = line.unwrap();
        if let Ok(log) = std::env::var("BUDDY_FIXTURE_LOG") {
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log).unwrap();
            writeln!(file,"{s}").unwrap();
        }
        if s.contains("\"method\":\"initialize\"") {emit(r#"{"id":1,"result":{}}"#);}
        else if s.contains("\"method\":\"model/list\"") {emit(r#"{"id":2,"result":{"data":[{"model":"fixture-model","displayName":"Fixture model (no inference)","isDefault":true,"hidden":false}]}}"#);}
        else if s.contains("\"method\":\"thread/start\"") {
            if s.contains("startup-delay") {std::thread::sleep(std::time::Duration::from_millis(1500));}
            emit(r#"{"id":2,"result":{"thread":{"id":"fixture-session"},"approvalPolicy":"on-request","approvalsReviewer":"user","sandbox":{"type":"readOnly"}}}"#);
        }
        else if s.contains("\"method\":\"turn/start\"") {
            emit(r#"{"id":3,"result":{"turn":{"id":"fixture-turn"}}}"#);
            emit(r#"{"method":"turn/started","params":{"threadId":"fixture-session","turn":{"id":"fixture-turn"}}}"#);
            if s.contains("Drop fixture") {return;}
            if !s.contains("Cancel fixture") {
                emit(r#"{"id":"approval-1","method":"item/commandExecution/requestApproval","params":{"threadId":"fixture-session","turnId":"fixture-turn","itemId":"cmd","kind":"command","command":"Get-ChildItem .","cwd":"fixture","reason":"Protocol fixture; no command is executed"}}"#);
            }
        }
        else if s.contains("\"id\":\"approval-1\"") && s.contains("\"decision\":\"accept\"") {
            emit(r#"{"id":"question-1","method":"item/tool/requestUserInput","params":{"threadId":"fixture-session","turnId":"fixture-turn","questions":[{"id":"q1","header":"Fixture","question":"Choose a fixture result","isSecret":false,"isOther":false,"options":[{"label":"A","description":"First fixture"},{"label":"B","description":"Second fixture"}]}]}}"#);
        }
        else if s.contains("\"id\":\"question-1\"") && s.contains("\"answers\"") {
            emit(r#"{"method":"item/completed","params":{"threadId":"fixture-session","turnId":"fixture-turn","item":{"type":"agentMessage","id":"answer","phase":"final_answer","text":"Fixture completed. No model or command was run."}}}"#);
            emit(r#"{"method":"turn/completed","params":{"threadId":"fixture-session","turn":{"id":"fixture-turn","status":"completed","error":null}}}"#);
            // Late notifications must never resurrect a terminal task.
            emit(r#"{"method":"item/agentMessage/delta","params":{"threadId":"fixture-session","turnId":"fixture-turn","itemId":"late","delta":"LATE SHOULD BE IGNORED"}}"#);
        }
        else if s.contains("\"method\":\"turn/interrupt\"") {
            emit(r#"{"method":"turn/completed","params":{"threadId":"fixture-session","turn":{"id":"fixture-turn","status":"interrupted","error":null}}}"#);
        }
    }
}
