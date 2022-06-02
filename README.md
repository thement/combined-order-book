Combined Order Book
===================

## Usage

Start server:
```shell
$ cargo run --bin server
    Finished dev [unoptimized + debuginfo] target(s) in 9.86s
     Running `target/debug/server`
connecting to exchanges
connected to binance
connected to bitstamp
starting grpc server
```

Then start client (connects to server on localhost):
```shell
$ cargo run --bin client
Summary { spread: -9.239999999997963, bids: [Level { exchange: "binance", price: 28036.1, amount: 0.16257 }, Level { exchange: "binance", price: 28031.48, amount: 0.01668 }, Level { exchange: "binance", price: 28031.47, amount: 0.0338 }, Level { exchange: "binance", price: 28031.34, amount: 0.03094 }, Level { exchange: "binance", price: 28031.33, amount: 0.07728 }, Level { exchange: "binance", price: 28029.08, amount: 0.5 }, Level { exchange: "binance", price: 28029.07, amount: 0.10271 }, Level { exchange: "binance", price: 28029.03, amount: 0.16701 }, Level { exchange: "binance", price: 28025.42, amount: 0.23465 }, Level { exchange: "binance", price: 28025.41, amount: 0.08779 }], asks: [Level { exchange: "bitstamp", price: 28026.86, amount: 0.16599976 }, Level { exchange: "bitstamp", price: 28029.53, amount: 0.06 }, Level { exchange: "bitstamp", price: 28030.71, amount: 0.25261016 }, Level { exchange: "bitstamp", price: 28030.72, amount: 0.33400425 }, Level { exchange: "bitstamp", price: 28032.56, amount: 0.06 }, Level { exchange: "bitstamp", price: 28034.6, amount: 0.269 }, Level { exchange: "bitstamp", price: 28034.9, amount: 0.83501061 }, Level { exchange: "binance", price: 28036.11, amount: 0.04637 }, Level { exchange: "bitstamp", price: 28036.4, amount: 0.37823327 }, Level { exchange: "bitstamp", price: 28036.42, amount: 0.06 }] }
Summary { spread: -10.899999999997817, bids: [Level { exchange: "binance", price: 28037.76, amount: 0.02231 }, Level { exchange: "binance", price: 28037.22, amount: 0.135 }, Level { exchange: "binance", price: 28036.14, amount: 0.01668 }, Level { exchange: "binance", price: 28036.1, amount: 0.13757 }, Level { exchange: "binance", price: 28031.49, amount: 0.0338 }, Level { exchange: "binance", price: 28031.34, amount: 0.03094 }, Level { exchange: "binance", price: 28031.33, amount: 0.07728 }, Level { exchange: "binance", price: 28029.08, amount: 0.5 }, Level { exchange: "binance", price: 28029.07, amount: 0.10271 }, Level { exchange: "binance", price: 28029.03, amount: 0.16701 }], asks: [Level { exchange: "bitstamp", price: 28026.86, amount: 0.16599976 }, Level { exchange: "bitstamp", price: 28029.53, amount: 0.06 }, Level { exchange: "bitstamp", price: 28030.71, amount: 0.25261016 }, Level { exchange: "bitstamp", price: 28030.72, amount: 0.33400425 }, Level { exchange: "bitstamp", price: 28032.56, amount: 0.06 }, Level { exchange: "bitstamp", price: 28034.6, amount: 0.269 }, Level { exchange: "bitstamp", price: 28034.9, amount: 0.83501061 }, Level { exchange: "bitstamp", price: 28036.4, amount: 0.37823327 }, Level { exchange: "bitstamp", price: 28036.42, amount: 0.06 }, Level { exchange: "binance", price: 28037.77, amount: 0.15168 }] }
...
```

## What's not very nice

- crypto-ws-client library is not very good
    - contains almost no error handling (cannot connect to exchange? panic)
    - the authors have been a bit confused about the behavior of rust async (thread sleep in async
      contexteetc)
    - mixing std::sync::mpsc and tokio::sync::mpsc channels
    - when currency pair is not available, no error is thrown, the update stream just never sends
      any updates
    - the API is ghastly
- no configuration, program is hardwired to fetch BTC/EUR from bitstamp and binance spot market 

## What's missing

- allmost any comments in code
- no proper currency rounding/data type support
- the way end-of-stream and exit of client/server is propagated might not be entirely correct
  (I have no time to debug that now)
- missing error handling of most errors
- missing handling of corner cases:

    - there's no way to represent errors and exceptional conditions in the protofile:
        - order book entries from some exchanges not available
        - order book entries from one exchange are older then from the other

    - if all exchanges return zero bids and asks, program will panic because it has no way of
      computing the spread


