# Bitcoind Rest API

> Request data from Bitcoind through rest api endpoints.

☠️⚠️ Work In Progress ⚠️☠️

## Install

> Add package to Cargo.toml file

```shell
$ cargo install bitcoind-rest-api
```

## Setup:

> Must have these environment variable set for the terminal to work. Could go in your `.zshrc` or `.bashrc`:

```shell
export BITCOIND_PASSWORD="..."
export BITCOIND_USERNAME="..."
export BITCOIND_URL="127.0.0.1:8332"
```

## Start Server

To start server at the default host and port of `127.0.0.1:3030`, run:

```shell
 $ bitcoind-rest-api
```

To start server at a specified port, pass the PORT argument

```shell
 $ bitcoind-rest-api 3031
```

> Could optionally pass the environment variable to the script:

```shell
 BITCOIND_PASSWORD=... BITCOIND_USERNAME=...BITCOIND_URL=... bitcoin-terminal-dashboard
```

## Endpoints

The endpoints used should map directly to rpc commands and parameters, where the command name is the url path and the arguments are query params. If a rpc argument is optional, then the query param is also optional - and same for required arguments. For example, the [getchaintxstats](https://developer.bitcoin.org/reference/rpc/getchaintxstats.html) command takes two arguments, `nblocks` and `blockhash`, so the url path to request the same information would be `localhost:3030/api/v1/getchaintxstats?nblocks={...}&blockhash={...}`

The following endpoints have been implemented:

---

[getblockcount](https://developer.bitcoin.org/reference/rpc/getblockcount.html)

```
GET /api/v1/getblockcount
```

---

[getblockstats](https://developer.bitcoin.org/reference/rpc/getblockstats.html)

```
GET /api/v1/getblockstats?hash_or_height={blockhash or height}
```

---

[getchaintxstats](https://developer.bitcoin.org/reference/rpc/getchaintxstats.html)

```
GET /api/v1/getchaintxstats?n_blocks={nblocks}&blockhash={blockhash}
```

---

[getdifficutly](https://developer.bitcoin.org/reference/rpc/getdifficutly.html)

```
GET /api/v1/getdifficutly
```

---

[getnetworkhashps](https://developer.bitcoin.org/reference/rpc/getnetworkhashps.html)

```
GET /api/v1/getnetworkhashps?n_blocks={nblocks}&height={height}
```

---

[gettxoutsetinfo](https://developer.bitcoin.org/reference/rpc/gettxoutsetinfo.html)

> Not: This will take a few seconds (or longer) to return

```
GET /api/v1/gettxoutsetinfo?hash_type={hash_type}
```

---

## Related

- [bitcoind-request](https://github.com/joegesualdo/bitcoind-request) - Type-safe wrapper around bitcoind RPC commands
- [bitcoin-node-query](https://github.com/joegesualdo/bitcoin-node-query) - Query information about the bitcoin network

## License

MIT © [Joe Gesualdo]()
