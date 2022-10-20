use bitcoind_request::{
    client::Client as BitcoindRequestClient,
    command::{
        get_block_count::GetBlockCountCommand,
        get_block_stats::{
            GetBlockStatsCommand, GetBlockStatsCommandResponse, TargetBlockArgument,
        },
        get_chain_tx_stats::GetChainTxStatsCommand,
        get_difficulty::GetDifficultyCommand,
        get_network_hash_ps::GetNetworkHashPsCommand,
        get_tx_out_set_info::GetTxOutSetInfoCommand,
        CallableCommand,
    }, Blockhash,
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
    let client = BitcoindRequestClient::new(&url, &username, &password).expect("failed to create client");
    client
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let frontend_development_url = "http://127.0.0.1:5173";
    let cors = warp::cors().allow_origin(frontend_development_url);

    let dashboard = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("dashboard"))
        .map(|| {
            let bitcoind_request_client = get_client();
            let block_count_result = GetBlockCountCommand::new().call(&bitcoind_request_client);
            let block_count = block_count_result.unwrap().0;
            let block_stats_response_result_for_current_height =
                GetBlockStatsCommand::new(TargetBlockArgument::Height(block_count))
                    .call(&bitcoind_request_client);
            let chain_tx_stats_result =
                GetChainTxStatsCommand::new().call(&bitcoind_request_client);
            let difficulty_response_result =
                GetDifficultyCommand::new().call(&bitcoind_request_client);
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
                    GetBlockStatsCommandResponse::SelectiveStats(response) => {
                        response.subsidy.unwrap()
                    }
                };
            let time_of_last_difficulty_adjustment_block =
                match block_stats_response_result_for_last_difficulty_adjustment_block.unwrap() {
                    GetBlockStatsCommandResponse::AllStats(response) => response.time,
                    GetBlockStatsCommandResponse::SelectiveStats(response) => {
                        response.time.unwrap()
                    }
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
            let blocks_until_retarget =
                percent_of_epoch_to_go * (BLOCKS_PER_DIFFICULTY_PERIOD as f64);
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
    let get_block_count_path = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("getblockcount"))
        .map(|| {
            let bitcoind_request_client = get_client();
            let block_count_result = GetBlockCountCommand::new().call(&bitcoind_request_client);
            let block_count = block_count_result.unwrap().0;
            warp::reply::json(&block_count)
            });
    let get_block_stats_path = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("getblockstats"))
        .and(warp::path::param())
        .map(|block_height_or_blockhash: String| {
            println!("{}", block_height_or_blockhash);
            let arg = if block_height_or_blockhash.chars().count() == 256 {
                TargetBlockArgument::Hash(Blockhash(block_height_or_blockhash))
            } else {
                TargetBlockArgument::Height(block_height_or_blockhash.parse::<u64>().unwrap())
            };
            let bitcoind_request_client = get_client();

            let block_stats_response_result=
                GetBlockStatsCommand::new(arg)
                    .call(&bitcoind_request_client);
            let block_stats = block_stats_response_result.unwrap();
            warp::reply::json(&block_stats)
            });
    let get_chain_tx_stats_root_path = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("getchaintxstats"))
        .map(|| {
            let bitcoind_request_client = get_client();

            let chain_tx_stats_result =
                GetChainTxStatsCommand::new().call(&bitcoind_request_client);
            let chain_tx_stats = chain_tx_stats_result.unwrap();
            warp::reply::json(&chain_tx_stats)
            });
    let get_chain_tx_stats_path = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("getchaintxstats"))
        .and(warp::path::param())
        .map(|blockhash: String| {
            let bitcoind_request_client = get_client();

            let chain_tx_stats_result =
                GetChainTxStatsCommand::new().set_blockhash(Blockhash(blockhash)).call(&bitcoind_request_client);
            let chain_tx_stats = chain_tx_stats_result.unwrap();
            warp::reply::json(&chain_tx_stats)
            });
    let get_difficulty_path = warp::get()
        .and(warp::path("api"))
        .and(warp::path("v1"))
        .and(warp::path("getdifficulty"))
        .map(|| {
            let bitcoind_request_client = get_client();
            let difficulty_response_result =
                GetDifficultyCommand::new().call(&bitcoind_request_client);
            let difficulty_response = difficulty_response_result.unwrap();
            let difficulty = difficulty_response.0;

            warp::reply::json(&difficulty)
            });

    let root = warp::path::end().map(|| "Welcome!");
    let routes = root
        .or(dashboard)
        .or(get_block_count_path)
        .or(get_block_stats_path)
        .or(get_chain_tx_stats_root_path)
        .or(get_chain_tx_stats_path)
        .or(get_difficulty_path)
        .with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

