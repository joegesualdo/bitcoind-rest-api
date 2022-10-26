use bitcoind_request::{
    client::Client as BitcoindRequestClient,
    command::{
        get_block_count::GetBlockCountCommand,
        get_block_stats::{
            GetBlockStatsCommand, GetBlockStatsCommandResponse, TargetBlockArgument, StatsArgumentChoices,
        },
        get_chain_tx_stats::GetChainTxStatsCommand,
        get_difficulty::GetDifficultyCommand,
        get_network_hash_ps::GetNetworkHashPsCommand,
        get_tx_out_set_info::GetTxOutSetInfoCommand,
        CallableCommand, get_block_hash::GetBlockHashCommand, get_block::{GetBlockCommand, GetBlockCommandVerbosity},
    },
    Blockhash,
};
use serde::{Deserialize, Serialize};
use std::env;

use warp::Filter;

const BLOCKS_PER_DIFFICULTY_PERIOD: u64 = 2016;

#[derive(Deserialize, Serialize)]
struct Response {
    price: f64,
    block_count: u64,
    total_money_supply: f64,
    time_of_last_block: u64,
    total_transactions_count: u64,
    tps_30days: f64,
    difficulty: f64,
    current_difficulty_epoch: u64,
    blocks_until_retarget: f64,
    average_seconds_per_block_for_current_epoch: u64,
    estimated_seconds_until_retarget: f64,
    estimated_hash_rate_for_last_2016_blocks: f64,
    subsidy_in_sats_at_current_block_height: u64,
}

fn get_client() -> BitcoindRequestClient {
    let password = env::var("BITCOIND_PASSWORD").expect("BITCOIND_PASSWORD env variable not set");
    let username = env::var("BITCOIND_USERNAME").expect("BITCOIND_USERNAME env variable not set");
    let url = env::var("BITCOIND_URL").expect("BITCOIND_URL env variable not set");
    let client =
        BitcoindRequestClient::new(&url, &username, &password).expect("failed to create client");
    client
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let port_arg = args.get(1);
    let default_port = 3030;
    let port = match port_arg {
        Some(port) => port.parse().unwrap(),
        None => default_port
    };
    pretty_env_logger::init();

    let cors = warp::cors().allow_any_origin();

    let dashboard = warp::get().and(warp::path("dashboard")).map(|| {
        let bitcoind_request_client = get_client();
        let block_count_result = GetBlockCountCommand::new().call(&bitcoind_request_client);
        let block_count = block_count_result.unwrap().0;
        let block_stats_response_result_for_current_height =
            GetBlockStatsCommand::new(TargetBlockArgument::Height(block_count))
                .call(&bitcoind_request_client);
        let chain_tx_stats_result = GetChainTxStatsCommand::new().call(&bitcoind_request_client);
        let difficulty_response_result = GetDifficultyCommand::new().call(&bitcoind_request_client);
        let current_difficulty_epoch = (block_count / BLOCKS_PER_DIFFICULTY_PERIOD) + 1;
        let block_height_of_last_difficulty_adjustment = (current_difficulty_epoch - 1) * 2016;
        let block_stats_response_result_for_last_difficulty_adjustment_block =
            GetBlockStatsCommand::new(TargetBlockArgument::Height(
                block_height_of_last_difficulty_adjustment,
            ))
            .call(&bitcoind_request_client);
        let estimated_hash_rate_response_result_for_last_2016_blocks =
            GetNetworkHashPsCommand::new()
                .set_n_blocks(
                    bitcoind_request::command::get_network_hash_ps::BlocksToIncludeArg::NBlocks(
                        2016,
                    ),
                )
                .call(&bitcoind_request_client);

        let time_of_last_block = match &block_stats_response_result_for_current_height
            .as_ref()
            .unwrap()
        {
            GetBlockStatsCommandResponse::AllStats(response) => response.time,
            GetBlockStatsCommandResponse::SelectiveStats(response) => response.time.unwrap(),
        };
        let subsidy_in_sats_at_current_block_height =
            match &block_stats_response_result_for_current_height.unwrap() {
                GetBlockStatsCommandResponse::AllStats(response) => response.subsidy,
                GetBlockStatsCommandResponse::SelectiveStats(response) => response.subsidy.unwrap(),
            };
        let time_of_last_difficulty_adjustment_block =
            match block_stats_response_result_for_last_difficulty_adjustment_block.unwrap() {
                GetBlockStatsCommandResponse::AllStats(response) => response.time,
                GetBlockStatsCommandResponse::SelectiveStats(response) => response.time.unwrap(),
            };
        // This defaults to getting about 30 days worth of of data
        let estimated_hash_rate_for_last_2016_blocks =
            estimated_hash_rate_response_result_for_last_2016_blocks
                .unwrap()
                .0;

        let chain_tx_stats = chain_tx_stats_result.unwrap();
        let difficulty_response = difficulty_response_result.unwrap();
        let difficulty = difficulty_response.0;

        let total_transactions_count = chain_tx_stats.txcount;

        let seconds_in_interval = chain_tx_stats.window_interval;
        let transactions_count_in_window = chain_tx_stats.window_tx_count as f64;
        let elapsed_seconds_in_window = seconds_in_interval as f64;
        let tps_30days = transactions_count_in_window / elapsed_seconds_in_window;

        let percent_of_epoch_complete: f64 =
            (block_count as f64 / BLOCKS_PER_DIFFICULTY_PERIOD as f64) % 1.0;
        let percent_of_epoch_to_go: f64 = 1.0 - percent_of_epoch_complete;
        let blocks_until_retarget = percent_of_epoch_to_go * (BLOCKS_PER_DIFFICULTY_PERIOD as f64);
        let blocks_since_last_retarget =
            BLOCKS_PER_DIFFICULTY_PERIOD as f64 - blocks_until_retarget;

        let duration_since_last_difficulty_adjustment =
            time_of_last_block - time_of_last_difficulty_adjustment_block;
        let average_seconds_per_block_for_current_epoch =
            duration_since_last_difficulty_adjustment / blocks_since_last_retarget as u64;
        let estimated_seconds_until_retarget = 10.0 * 60.0 * blocks_until_retarget;

        // HARDCODED
        let price = 22122.0;
        let total_money_supply = 70000.1;
        // let tx_out_set_info = GetTxOutSetInfoCommand::new().call(&bitcoind_request_client);
        // let total_money_supply = tx_out_set_info.unwrap().total_amount;

        let response = Response {
            price,
            block_count,
            total_money_supply,
            time_of_last_block,
            total_transactions_count,
            tps_30days,
            difficulty,
            current_difficulty_epoch,
            blocks_until_retarget,
            average_seconds_per_block_for_current_epoch,
            estimated_seconds_until_retarget,
            estimated_hash_rate_for_last_2016_blocks,
            subsidy_in_sats_at_current_block_height,
        };
        warp::reply::json(&response)
    });
    // /api/v1/getblockcount
    let get_block_count_path = warp::get().and(warp::path("getblockcount")).map(|| {
        let bitcoind_request_client = get_client();
        let block_count_result = GetBlockCountCommand::new().call(&bitcoind_request_client);
        let block_count = block_count_result.unwrap().0;
        warp::reply::json(&block_count)
    });
    #[derive(Deserialize)]
    struct GetBlockStatsQueryParams {
        hash_or_height: String,
        stats: Option<Vec<String>>
    }
    // /api/v1/getblockstats?hash_or_height={blockhash or height}
    let get_block_stats_path = warp::get()
        .and(warp::path("getblockstats"))
        .and(warp::query::<GetBlockStatsQueryParams>())
        .map(|query_params: GetBlockStatsQueryParams|  {
            let hash_or_height = query_params.hash_or_height;
            let arg = if hash_or_height.chars().count() == 64 {
                TargetBlockArgument::Hash(Blockhash(hash_or_height))
            } else {
                TargetBlockArgument::Height(hash_or_height.parse::<u64>().unwrap())
            };

            let bitcoind_request_client = get_client();
            let command_with_hash_or_height_set = GetBlockStatsCommand::new(arg);
            let command_with_hash_or_height_and_stats_set = match query_params.stats{
                Some(stats) => {
                    todo!()
                },
                None => command_with_hash_or_height_set
            };

            let block_stats_response_result = command_with_hash_or_height_and_stats_set.call(&bitcoind_request_client);
            let block_stats = block_stats_response_result.unwrap();
            warp::reply::json(&block_stats)
        });
    #[derive(Deserialize)]
    struct GetTxOutSetInfoQueryParams{
        hash_type: Option<String>,
    }
    // /api/v1/gettxoutsetinfo?hash_type={hash_type}
    let get_tx_out_set_info_path = warp::get()
        .and(warp::path("gettxoutsetinfo"))
        .and(warp::query::<GetTxOutSetInfoQueryParams>())
        .map(|query_params: GetTxOutSetInfoQueryParams|  {
            let bitcoind_request_client = get_client();
            let command = GetTxOutSetInfoCommand::new();
            let command_with_hash_type_set = match query_params.hash_type{
                Some(hash_type) => {
                    todo!()
                },
                None => command
            };

            let get_tx_outset_info_response_result = command_with_hash_type_set.call(&bitcoind_request_client);
            let tx_out_set_info = get_tx_outset_info_response_result.unwrap();
            warp::reply::json(&tx_out_set_info)
        });

    #[derive(Deserialize)]
    struct GetChainTxStatsQueryParams {
       n_blocks: Option<u64>,
       blockhash: Option<String>
    }
    // /api/v1/getchaintxstats?n_blocks=10000&blockhash=00000000770ebe897270ca5f6d539d8afb4ea4f4e757761a34ca82e17207d886
    let get_chain_tx_stats_path =
        warp::path("getchaintxstats")
        .and(warp::query::<GetChainTxStatsQueryParams>())
        .map(|query_params: GetChainTxStatsQueryParams|  {
            let bitcoind_request_client = get_client();

            let command = GetChainTxStatsCommand::new();
            let command_with_n_blocks_set = match query_params.n_blocks {
                Some(n_blocks) => 
                    command.set_n_blocks(n_blocks),
                None => command
            };
            let command_with_n_blocks_and_blockhash_set = match query_params.blockhash{
                Some(blockhash) => 
                    command_with_n_blocks_set.set_blockhash(Blockhash(blockhash)),
                None => command_with_n_blocks_set
            };

            let chain_tx_stats_result =
                command_with_n_blocks_and_blockhash_set.call(&bitcoind_request_client);
            let chain_tx_stats = chain_tx_stats_result.unwrap();
            warp::reply::json(&chain_tx_stats)
        });
    // /api/v1/getdifficutly
    let get_difficulty_path = warp::get().and(warp::path("getdifficulty")).map(|| {
        let bitcoind_request_client = get_client();
        let difficulty_response_result = GetDifficultyCommand::new().call(&bitcoind_request_client);
        let difficulty_response = difficulty_response_result.unwrap();
        let difficulty = difficulty_response.0;

        warp::reply::json(&difficulty)
    });
    #[derive(Deserialize)]
    struct GetBlockhashQueryParams{
        height: u64,
    }
    // /api/v1/getblockhash?height={height}
    let get_blockhash_path = warp::get()
        .and(warp::path("getblockhash"))
        .and(warp::query::<GetBlockhashQueryParams>())
        .map(|query_params: GetBlockhashQueryParams|  {
            let bitcoind_request_client = get_client();
            let command = GetBlockHashCommand::new(query_params.height);
            let get_blockhash_response_result = command.call(&bitcoind_request_client);
            let blockhash = get_blockhash_response_result.unwrap();
            warp::reply::json(&blockhash)
        });

    #[derive(Deserialize)]
    struct GetNetworkHashPsQueryParams {
        n_blocks: Option<u64>,
            height: Option<u64>
    }
    // /api/v1/getnetworkhashps?n_blocks=2016&height=200000
    let get_network_hash_ps_path = 
        warp::get()
        .and(warp::path("getnetworkhashps"))
        .and(warp::path::end())
        .and(warp::query::<GetNetworkHashPsQueryParams>())
        .map(|query_params: GetNetworkHashPsQueryParams|  {
            let bitcoind_request_client = get_client();
            let command = GetNetworkHashPsCommand::new();
            let command_with_n_blocks_set = match query_params.n_blocks {
                Some(n_blocks) => 
                    command.set_n_blocks(
                        bitcoind_request::command::get_network_hash_ps::BlocksToIncludeArg::NBlocks(
                            n_blocks
                        ),
                    ),
                None => command
            };
            let command_with_n_blocks_and_height_set = match query_params.height {
                Some(height) => 
                    command_with_n_blocks_set.set_height(
                        bitcoind_request::command::get_network_hash_ps::HeightArg::Height(height)
                    ),
                None => command_with_n_blocks_set
            };
            let estimated_hash_rate_response_result_for_last_2016_blocks = command_with_n_blocks_and_height_set.call(&bitcoind_request_client);
            let estimated_hash_rate_for_last_2016_blocks =
                estimated_hash_rate_response_result_for_last_2016_blocks
                    .unwrap()
                    .0;
            warp::reply::json(&estimated_hash_rate_for_last_2016_blocks)
      });
    #[derive(Deserialize)]
    struct GetBlockQueryParams {
        blockhash: String,
            verbosity: Option<u64>
    }
    // /api/v1/getnetworkhashps?n_blocks=2016&height=200000
    let get_block_path = 
        warp::get()
        .and(warp::path("getblock"))
        .and(warp::path::end())
        .and(warp::query::<GetBlockQueryParams>())
        .map(|query_params: GetBlockQueryParams|  {
            let bitcoind_request_client = get_client();
            let command = GetBlockCommand::new(Blockhash(query_params.blockhash));
            let command_with_verbosity_set = match query_params.verbosity{
                Some(verbosity) => 
                    match verbosity {
                        0 => command.verbosity(GetBlockCommandVerbosity::SerializedHexEncodedData),
                        1 => command.verbosity(GetBlockCommandVerbosity::BlockObjectWithoutTransactionInformation),
                        2 => command.verbosity(GetBlockCommandVerbosity::BlockObjectWithTransactionInformation),
                        _ => panic!("verbosity {} not supported", verbosity)
                    },
                None => command
            };
            let get_block_response_result= command_with_verbosity_set.call(&bitcoind_request_client);
            let block =
               get_block_response_result.unwrap();
            warp::reply::json(&block)
      });

    // /
    let root = warp::path::end().map(|| "Welcome!");
    let api_v1_path = warp::path("api").and(warp::path("v1"));
    let routes = root
        .or(api_v1_path.and(dashboard))
        .or(api_v1_path.and(get_network_hash_ps_path))
        .or(api_v1_path.and(get_block_path))
        .or(api_v1_path.and(get_blockhash_path))
        .or(api_v1_path.and(get_block_count_path))
        .or(api_v1_path.and(get_block_stats_path))
        .or(api_v1_path.and(get_chain_tx_stats_path))
        .or(api_v1_path.and(get_difficulty_path))
        .or(api_v1_path.and(get_tx_out_set_info_path))
        .with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}
