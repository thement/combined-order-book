pub mod pb {
    tonic::include_proto!("orderbook");
}

use futures::Stream;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use pb::{Empty, Summary};

type EmptyResult<T> = Result<Response<T>, Status>;
type OrderbookStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send>>;

#[derive(Debug)]
pub struct OrderbookAggregatorServer {
    sequence_receiver: broadcast::Receiver<u32>,
    sequence_sender: broadcast::Sender<u32>,
}

#[tonic::async_trait]
impl pb::orderbook_aggregator_server::OrderbookAggregator for OrderbookAggregatorServer {
    type BookSummaryStream = OrderbookStream;

    async fn book_summary(&self, req: Request<Empty>) -> EmptyResult<Self::BookSummaryStream> {
        println!("OrderbookAggregatorServer::server_streaming_echo");
        println!("\tclient connected from: {:?}", req.remote_addr());

        let mut sequence_receiver = self.sequence_sender.subscribe();

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            loop {
                match sequence_receiver.recv().await {
                    Ok(sequence_number) => {
                        let item = Summary {
                            spread: sequence_number as f64,
                            bids: vec![],
                            asks: vec![],
                        };
                        match tx.send(Result::<_, Status>::Ok(item)).await {
                            Ok(_) => {
                                // item (server response) was queued to be send to client
                            }
                            Err(_item) => {
                                // output_stream was build from rx and both are dropped
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => (),
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            println!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::BookSummaryStream
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (sequence_sender, sequence_receiver) = broadcast::channel(1);
    let sequence_sender_inner = sequence_sender.clone();
    tokio::spawn(async move {
        let mut sequence_number = 0;
        loop {
            if sequence_sender_inner.send(sequence_number).is_err() {
                // All clients dropped
                break;
            }
            sequence_number += 1;
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
    });
    let server = OrderbookAggregatorServer {
        sequence_sender,
        sequence_receiver,
    };
    Server::builder()
        .add_service(pb::orderbook_aggregator_server::OrderbookAggregatorServer::new(server))
        .serve("[::1]:54321".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}
