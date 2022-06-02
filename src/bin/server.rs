use combined_order_book;
pub mod pb {
    tonic::include_proto!("orderbook");
}

use futures::Stream;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin, time::Duration};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use pb::{Empty, Level, Summary};

type EmptyResult<T> = Result<Response<T>, Status>;
type OrderbookStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send>>;

#[derive(Debug)]
pub struct OrderbookAggregatorServer {
    watcher: combined_order_book::SpreadScraperWatcher,
}

fn convert_orderbook_entries_to_proto(
    entries: &[combined_order_book::OrderBookEntry],
) -> Vec<Level> {
    entries
        .iter()
        .map(|entry| Level {
            price: entry.price,
            amount: entry.amount,
            exchange: entry.source.into(),
        })
        .collect()
}

#[tonic::async_trait]
impl pb::orderbook_aggregator_server::OrderbookAggregator for OrderbookAggregatorServer {
    type BookSummaryStream = OrderbookStream;

    async fn book_summary(&self, req: Request<Empty>) -> EmptyResult<Self::BookSummaryStream> {
        println!("client connected from: {:?}", req.remote_addr());

        // Each watcher has to have it's own copy
        let mut watcher = self.watcher.clone();

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            while let Ok(summary) = watcher.receive().await {
                let item = Summary {
                    spread: summary.spread,
                    bids: convert_orderbook_entries_to_proto(&summary.order_book.bids),
                    asks: convert_orderbook_entries_to_proto(&summary.order_book.asks),
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
            println!("client disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::BookSummaryStream
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("connecting to exchanges");
    let watcher = combined_order_book::start().await;
    eprintln!("starting grpc server");
    let server = OrderbookAggregatorServer { watcher };
    Server::builder()
        .add_service(pb::orderbook_aggregator_server::OrderbookAggregatorServer::new(server))
        .serve("[::1]:54321".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}
