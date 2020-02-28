
use std::sync::{Arc, Mutex};
use futures::try_join;
use futures::stream::StreamExt;
use tokio::net::{TcpListener, TcpStream};

use crate::packet_handler::{Direction, PacketHandler};
use crate::pipe::Pipe;

#[derive(Debug)]
pub struct Server {
  db_addr: String,
  listener: TcpListener,
}

impl Server {
  pub async fn new(bind_addr: String, db_addr: String) -> Server {
    Server { 
      db_addr: db_addr,
      listener: TcpListener::bind(bind_addr).await.unwrap(),
    }
  }

  pub async fn run<T: PacketHandler+Send+Sync+'static>(&mut self, packet_handler: T) {
    let packet_handler = Arc::new(Mutex::new(packet_handler));
    let mut incoming = self.listener.incoming();
    while let Some(conn) = incoming.next().await {
      let _ = match conn {
        Ok(client_socket) => {
          info!("Accepted connection from {:?}", client_socket.peer_addr());
          let db_addr = self.db_addr.clone();
          let handler_ref = packet_handler.clone();
          tokio::spawn(async move {
            let client_socket = Arc::new(client_socket);
            let server_socket = Arc::new(TcpStream::connect(db_addr).await.unwrap());
            let mut forward_pipe = Pipe::new(String::from("forward"), handler_ref.clone(), Direction::Forward, client_socket.clone(), server_socket.clone());
            let mut backward_pipe = Pipe::new(String::from("backward"), handler_ref.clone(), Direction::Backward, server_socket.clone(), client_socket.clone());
            let _ = match try_join!(forward_pipe.run(), backward_pipe.run()) {
              Ok(((),())) => { trace!("Pipe closed successfully"); },
              Err(e) => { error!("Pipe closed with error: {}", e); },
            };
            info!("Closing connection from {:?}", client_socket.peer_addr());
          });
        }
        Err(err) => {
            // Handle error by printing to STDOUT.
            error!("accept error = {:?}", err);
        }
      };
    }
    info!("Server run() complete");
  }
}
