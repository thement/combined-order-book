pub mod pb {
    tonic::include_proto!("orderbook");
}

use tokio_stream::StreamExt;
use tonic::transport::Channel;

use pb::{orderbook_aggregator_client::OrderbookAggregatorClient, Empty};

async fn streaming_orderbook(client: &mut OrderbookAggregatorClient<Channel>) {
    let mut stream = client.book_summary(Empty {}).await.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        println!("{:?}", item.unwrap());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = OrderbookAggregatorClient::connect("http://[::1]:54321")
        .await
        .unwrap();

    streaming_orderbook(&mut client).await;

    Ok(())
}
