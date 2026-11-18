use htmx_analyzer::{ExitKind, Server};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::InitializeParams;
use std::time::Duration;

#[allow(dead_code)]
pub struct TestClient {
    client_conn: Connection,
    handle: Option<std::thread::JoinHandle<ExitKind>>,
    next_id: i32,
}

#[allow(dead_code)]
impl TestClient {
    pub fn start(init_options: serde_json::Value) -> Self {
        Self::start_with_queued(init_options, Vec::new())
    }

    pub fn start_with_queued(init_options: serde_json::Value, queued: Vec<Message>) -> Self {
        let (server_conn, client_conn) = Connection::memory();
        let init_req = Request::new(
            RequestId::from(1),
            "initialize".to_string(),
            serde_json::to_value(InitializeParams {
                initialization_options: Some(init_options),
                ..Default::default()
            })
            .unwrap(),
        );
        client_conn.sender.send(Message::Request(init_req)).unwrap();

        let initialized_not = Notification::new("initialized".to_string(), serde_json::json!({}));
        client_conn
            .sender
            .send(Message::Notification(initialized_not))
            .unwrap();

        for msg in queued {
            client_conn.sender.send(msg).unwrap();
        }

        let server = Server::new(server_conn);
        let handle = std::thread::spawn(move || server.run().expect("server run failed"));

        // Read the initialize response from server
        let resp = client_conn
            .receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("Failed to receive initialize response");
        match resp {
            Message::Response(r) if r.id == RequestId::from(1) => {}
            other => panic!("Expected initialize response, got {:?}", other),
        }

        Self {
            client_conn,
            handle: Some(handle),
            next_id: 2,
        }
    }

    pub fn request(&mut self, method: &str, params: serde_json::Value) -> Response {
        let id = RequestId::from(self.next_id);
        self.next_id += 1;
        let req = Request::new(id.clone(), method.to_string(), params);
        self.client_conn.sender.send(Message::Request(req)).unwrap();

        loop {
            match self
                .client_conn
                .receiver
                .recv_timeout(Duration::from_secs(5))
            {
                Ok(Message::Response(resp)) if resp.id == id => return resp,
                Ok(_) => continue,
                Err(e) => panic!("Timed out waiting for response to {}: {:?}", method, e),
            }
        }
    }

    pub fn notify(&self, method: &str, params: serde_json::Value) {
        let not = Notification::new(method.to_string(), params);
        self.client_conn
            .sender
            .send(Message::Notification(not))
            .unwrap();
    }

    pub fn recv(&self, timeout: Duration) -> Option<Message> {
        self.client_conn.receiver.recv_timeout(timeout).ok()
    }

    pub fn join(mut self) -> ExitKind {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("server thread panicked")
        } else {
            ExitKind::Clean
        }
    }
}
