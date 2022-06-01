use crypto_market_type::MarketType;
use crypto_ws_client::{BinanceSpotWSClient, BitstampWSClient, WSClient};
use std::collections::HashMap;
use std::future::Future;
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
struct OrderBookEntry {
    source: &'static str,
    price: f64,
    amount: f64,
}

impl OrderBookEntry {
    fn from_crypto_order(source: &'static str, order: &crypto_msg_parser::Order) -> Self {
        Self {
            source,
            price: order.price,
            amount: order.quantity_base,
        }
    }
}

#[derive(Clone, Debug)]
struct OrderBook {
    asks: Vec<OrderBookEntry>,
    bids: Vec<OrderBookEntry>,
}

impl OrderBook {
    fn new() -> Self {
        Self {
            asks: vec![],
            bids: vec![],
        }
    }

    fn append(&mut self, other: &OrderBook) {
        self.asks.extend_from_slice(&other.asks);
        self.bids.extend_from_slice(&other.bids);
    }

    fn sort(&mut self) {
        // Lowest ask at the top
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        // Highest bid at the top
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
    }

    fn spread(&self) -> f64 {
        assert!(self.asks.len() >= 1);
        assert!(self.bids.len() >= 1);

        self.asks[0].price - self.bids[0].price
    }

    fn print_top_10(&self) {
        println!("spread: {}", self.spread());
        println!("top 10 asks:");
        for ask in self.asks.iter().take(10) {
            println!("  {}, {}, {}", ask.source, ask.price, ask.amount);
        }
        println!("top 10 bids:");
        for bid in self.bids.iter().take(10) {
            println!("  {}, {}, {}", bid.source, bid.price, bid.amount);
        }
    }

    fn convert_orders(
        source: &'static str,
        orders: &[crypto_msg_parser::Order],
    ) -> Vec<OrderBookEntry> {
        orders
            .iter()
            .map(|order| OrderBookEntry::from_crypto_order(source, order))
            .collect()
    }

    fn from_asks_and_bids(
        source: &'static str,
        asks: &[crypto_msg_parser::Order],
        bids: &[crypto_msg_parser::Order],
    ) -> Self {
        Self {
            asks: Self::convert_orders(source, asks),
            bids: Self::convert_orders(source, bids),
        }
    }
}

type ChanOrderBook = (&'static str, OrderBook);

fn parse_exchange_messages_thread(
    exchange_name: &'static str,
    rx_message: std_mpsc::Receiver<String>,
    tx_order_book: mpsc::UnboundedSender<ChanOrderBook>,
) {
    for message in rx_message {
        eprintln!("received = {}", message);
        let update =
            crypto_msg_parser::parse_l2_topk(exchange_name, MarketType::Spot, &message, Some(0));
        match update {
            Err(e) => eprintln!("got error: {}", e),
            Ok(update) => {
                assert_eq!(update.len(), 1, "only one update per message expected");
                let u = &update[0];
                let order_book = OrderBook::from_asks_and_bids(exchange_name, &u.asks, &u.bids);
                if tx_order_book.send((exchange_name, order_book)).is_err() {
                    // Shutdown, other side exited
                    break;
                }
            }
        }
    }
}

struct SpreadScraper {
    tx_order_book: mpsc::UnboundedSender<ChanOrderBook>,
    rx_order_book: mpsc::UnboundedReceiver<ChanOrderBook>,
    order_book_map: HashMap<&'static str, Option<OrderBook>>,
}

impl SpreadScraper {
    fn new() -> Self {
        let (tx_order_book, rx_order_book) = mpsc::unbounded_channel();
        Self {
            tx_order_book,
            rx_order_book,
            order_book_map: HashMap::new(),
        }
    }

    async fn connect_to_exchange<T, F, U>(
        &mut self,
        pair_name: &'static str,
        exchange_name: &'static str,
        ws_client_builder: F,
    ) where
        F: FnOnce(std_mpsc::Sender<String>) -> U,
        U: Future<Output = T>,
        T: WSClient + Send + Sync + 'static,
    {
        let (tx_message, rx_message) = std_mpsc::channel();
        let ws_client = ws_client_builder(tx_message).await;
        eprintln!("connected to {}", exchange_name);

        tokio::task::spawn(async move {
            let symbols = vec![pair_name.to_string()];
            ws_client.subscribe_orderbook_topk(&symbols).await;
            ws_client.run().await;
            ws_client.close();
        });

        let tx_order_book = self.tx_order_book.clone();
        std::thread::spawn(move || {
            parse_exchange_messages_thread(exchange_name, rx_message, tx_order_book);
        });

        self.order_book_map.insert(exchange_name, None);
    }

    async fn collect_and_sort_order_books(&mut self) {
        let mut combined_order_book = OrderBook::new();

        for (exchange, maybe_order_book) in self.order_book_map.iter() {
            match maybe_order_book {
                Some(order_book) => combined_order_book.append(order_book),
                None => {
                    eprintln!("not all exchanges available yet");
                    return;
                }
            }
        }

        combined_order_book.sort();

        combined_order_book.print_top_10();
    }

    async fn run(&mut self) {
        while let Some((exchange_name, order_book)) = self.rx_order_book.recv().await {
            self.order_book_map.insert(exchange_name, Some(order_book));
            self.collect_and_sort_order_books().await;
        }
    }
}

#[tokio::main]
async fn main() {
    // Hack to initialize OnceCell inside `crypto_pair`, otherwise it panics because it tries to
    // run blocking code in non-blocking (async) context
    tokio::task::spawn_blocking(|| crypto_pair::normalize_pair("BTCEUR", "binance"))
        .await
        .unwrap();

    let mut spread_scraper = SpreadScraper::new();

    spread_scraper
        .connect_to_exchange("BTCEUR", "binance", |tx| BinanceSpotWSClient::new(tx, None))
        .await;

    spread_scraper
        .connect_to_exchange("btceur", "bitstamp", |tx| BitstampWSClient::new(tx, None))
        .await;

    spread_scraper.run().await;
}