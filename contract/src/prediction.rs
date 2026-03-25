use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

use crate::config::{self, PERSISTENT_BUMP, PERSISTENT_THRESHOLD};
use crate::errors::InsightArenaError;
use crate::escrow;
use crate::storage_types::{DataKey, Market, Prediction, UserProfile};

// ── TTL helpers ───────────────────────────────────────────────────────────────

fn bump_prediction(env: &Env, market_id: u64, predictor: &Address) {
    env.storage().persistent().extend_ttl(
        &DataKey::Prediction(market_id, predictor.clone()),
        PERSISTENT_THRESHOLD,
        PERSISTENT_BUMP,
    );
}

fn bump_market(env: &Env, market_id: u64) {
    env.storage().persistent().extend_ttl(
        &DataKey::Market(market_id),
        PERSISTENT_THRESHOLD,
        PERSISTENT_BUMP,
    );
}

fn bump_predictor_list(env: &Env, market_id: u64) {
    env.storage().persistent().extend_ttl(
        &DataKey::PredictorList(market_id),
        PERSISTENT_THRESHOLD,
        PERSISTENT_BUMP,
    );
}

fn bump_user(env: &Env, address: &Address) {
    env.storage().persistent().extend_ttl(
        &DataKey::User(address.clone()),
        PERSISTENT_THRESHOLD,
        PERSISTENT_BUMP,
    );
}

// ── Event emission ────────────────────────────────────────────────────────────

fn emit_prediction_submitted(
    env: &Env,
    market_id: u64,
    predictor: &Address,
    outcome: &Symbol,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("pred"), symbol_short!("submitd")),
        (market_id, predictor.clone(), outcome.clone(), amount),
    );
}

// ── Entry-point logic ─────────────────────────────────────────────────────────

/// Submit a prediction for an open market by staking XLM on a chosen outcome.
///
/// Validation order:
/// 1. Platform not paused
/// 2. Predictor authorisation via `require_auth()`
/// 3. Market exists (else `MarketNotFound`)
/// 4. `current_time < market.end_time` (else `MarketExpired`)
/// 5. `chosen_outcome` is present in `market.outcome_options` (else `InvalidOutcome`)
/// 6. `stake_amount >= market.min_stake` (else `StakeTooLow`)
/// 7. `stake_amount <= market.max_stake` (else `StakeTooHigh`)
/// 8. Predictor has not already submitted a prediction for this market (else `AlreadyPredicted`)
///
/// On success:
/// - XLM is locked in escrow via `escrow::lock_stake`.
/// - A `Prediction` record is written to `DataKey::Prediction(market_id, predictor)`.
/// - `PredictorList(market_id)` is appended with the predictor address.
/// - `market.total_pool` and `market.participant_count` are updated atomically.
/// - The predictor's `UserProfile` stats are updated (or created on first prediction).
/// - A `PredictionSubmitted` event is emitted.
pub fn submit_prediction(
    env: &Env,
    predictor: Address,
    market_id: u64,
    chosen_outcome: Symbol,
    stake_amount: i128,
) -> Result<(), InsightArenaError> {
    // ── Guard 1: platform not paused ─────────────────────────────────────────
    config::ensure_not_paused(env)?;

    // ── Guard 2: predictor authorisation ─────────────────────────────────────
    predictor.require_auth();

    // ── Guard 3: market must exist ────────────────────────────────────────────
    let mut market: Market = env
        .storage()
        .persistent()
        .get(&DataKey::Market(market_id))
        .ok_or(InsightArenaError::MarketNotFound)?;

    // ── Guard 4: market must not be expired ───────────────────────────────────
    let now = env.ledger().timestamp();
    if now >= market.end_time {
        return Err(InsightArenaError::MarketExpired);
    }

    // ── Guard 5: chosen_outcome must be in outcome_options ───────────────────
    let outcome_valid = market.outcome_options.iter().any(|o| o == chosen_outcome);
    if !outcome_valid {
        return Err(InsightArenaError::InvalidOutcome);
    }

    // ── Guard 6 & 7: stake_amount must be within [min_stake, max_stake] ───────
    if stake_amount < market.min_stake {
        return Err(InsightArenaError::StakeTooLow);
    }
    if stake_amount > market.max_stake {
        return Err(InsightArenaError::StakeTooHigh);
    }

    // ── Guard 8: user has not already predicted on this market ────────────────
    let prediction_key = DataKey::Prediction(market_id, predictor.clone());
    if env.storage().persistent().has(&prediction_key) {
        return Err(InsightArenaError::AlreadyPredicted);
    }

    // ── Lock stake in escrow (transfer XLM from predictor to contract) ────────
    escrow::lock_stake(env, &predictor, stake_amount)?;

    // ── Store Prediction record ───────────────────────────────────────────────
    let prediction = Prediction::new(
        market_id,
        predictor.clone(),
        chosen_outcome.clone(),
        stake_amount,
        now,
    );
    env.storage().persistent().set(&prediction_key, &prediction);
    bump_prediction(env, market_id, &predictor);

    // ── Append predictor to PredictorList ────────────────────────────────────
    let list_key = DataKey::PredictorList(market_id);
    let mut predictors: Vec<Address> = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));
    predictors.push_back(predictor.clone());
    env.storage().persistent().set(&list_key, &predictors);
    bump_predictor_list(env, market_id);

    // ── Update market total_pool and participant_count atomically ─────────────
    market.total_pool = market
        .total_pool
        .checked_add(stake_amount)
        .ok_or(InsightArenaError::Overflow)?;
    market.participant_count = market
        .participant_count
        .checked_add(1)
        .ok_or(InsightArenaError::Overflow)?;
    env.storage()
        .persistent()
        .set(&DataKey::Market(market_id), &market);
    bump_market(env, market_id);

    // ── Update UserProfile stats (create profile on first prediction) ─────────
    let user_key = DataKey::User(predictor.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&user_key)
        .unwrap_or_else(|| UserProfile::new(predictor.clone(), now));

    profile.total_predictions = profile
        .total_predictions
        .checked_add(1)
        .ok_or(InsightArenaError::Overflow)?;
    profile.total_staked = profile
        .total_staked
        .checked_add(stake_amount)
        .ok_or(InsightArenaError::Overflow)?;

    env.storage().persistent().set(&user_key, &profile);
    bump_user(env, &predictor);

    // ── Emit PredictionSubmitted event ────────────────────────────────────────
    emit_prediction_submitted(env, market_id, &predictor, &chosen_outcome, stake_amount);

    Ok(())
}

/// Return the stored [`Prediction`] for a given `(market_id, predictor)` pair.
///
/// This is a read-only query — no state is mutated. The TTL of the prediction
/// record is extended on every successful read so it remains live while clients
/// are actively querying it.
///
/// # Errors
/// - `PredictionNotFound` — no prediction exists for the supplied key.
pub fn get_prediction(
    env: &Env,
    market_id: u64,
    predictor: Address,
) -> Result<Prediction, InsightArenaError> {
    let key = DataKey::Prediction(market_id, predictor.clone());

    let prediction: Prediction = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(InsightArenaError::PredictionNotFound)?;

    // Extend TTL so an active read keeps the record alive.
    bump_prediction(env, market_id, &predictor);

    Ok(prediction)
}

/// Check whether `predictor` has already submitted a prediction on
/// `market_id`.
///
/// This is a lightweight boolean check that does **not** load the full
/// `Prediction` struct — it only tests key existence in persistent storage.
/// No state mutations occur.
///
/// # Arguments
/// * `market_id`  — The market to query.
/// * `predictor`  — The address to check.
///
/// # Returns
/// `true` if a prediction exists, `false` otherwise. Never panics.
pub fn has_predicted(env: &Env, market_id: u64, predictor: Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Prediction(market_id, predictor))
}

/// Return all [`Prediction`] records for a given market.
///
/// Loads the `PredictorList(market_id)` (a `Vec<Address>` of every address
/// that called `submit_prediction` on this market), then fetches each
/// individual `Prediction` record. TTLs are extended for the predictor
/// list and every prediction accessed.
///
/// Returns an empty `Vec` if the market has no predictions or does not
/// exist.
///
/// # Arguments
/// * `market_id` — The market whose predictions to list.
pub fn list_market_predictions(env: &Env, market_id: u64) -> Vec<Prediction> {
    let list_key = DataKey::PredictorList(market_id);

    let predictors: Vec<Address> = env
        .storage()
        .persistent()
        .get(&list_key)
        .unwrap_or_else(|| Vec::new(env));

    if predictors.is_empty() {
        return Vec::new(env);
    }

    // Extend TTL for the predictor list itself.
    bump_predictor_list(env, market_id);

    let mut results: Vec<Prediction> = Vec::new(env);

    for predictor in predictors.iter() {
        let pred_key = DataKey::Prediction(market_id, predictor.clone());
        if let Some(prediction) = env.storage().persistent().get::<DataKey, Prediction>(&pred_key)
        {
            bump_prediction(env, market_id, &predictor);
            results.push_back(prediction);
        }
    }

    results
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod prediction_tests {
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::token::{Client as TokenClient, StellarAssetClient};
    use soroban_sdk::{symbol_short, vec, Address, Env, String};

    use crate::market::CreateMarketParams;
    use crate::{InsightArenaContract, InsightArenaContractClient, InsightArenaError};

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn register_token(env: &Env) -> Address {
        let token_admin = Address::generate(env);
        env.register_stellar_asset_contract_v2(token_admin)
            .address()
    }

    /// Deploy and initialise the contract; return client + xlm_token address.
    fn deploy(env: &Env) -> (InsightArenaContractClient<'_>, Address) {
        let id = env.register(InsightArenaContract, ());
        let client = InsightArenaContractClient::new(env, &id);
        let admin = Address::generate(env);
        let oracle = Address::generate(env);
        let xlm_token = register_token(env);
        env.mock_all_auths();
        client.initialize(&admin, &oracle, &200_u32, &xlm_token);
        (client, xlm_token)
    }

    fn default_params(env: &Env) -> CreateMarketParams {
        let now = env.ledger().timestamp();
        CreateMarketParams {
            title: String::from_str(env, "Will it rain?"),
            description: String::from_str(env, "Daily weather market"),
            category: symbol_short!("weather"),
            outcomes: vec![env, symbol_short!("yes"), symbol_short!("no")],
            end_time: now + 1000,
            resolution_time: now + 2000,
            creator_fee_bps: 100,
            min_stake: 10_000_000,
            max_stake: 100_000_000,
            is_public: true,
        }
    }

    /// Mint `amount` XLM stroops to `recipient` using the stellar asset client.
    fn fund(env: &Env, xlm_token: &Address, recipient: &Address, amount: i128) {
        StellarAssetClient::new(env, xlm_token).mint(recipient, &amount);
    }

    // ── submit_prediction tests ───────────────────────────────────────────────
    // ── Happy path ────────────────────────────────────────────────────────────

    #[test]
    fn submit_prediction_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);

        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );

        // Verify prediction stored correctly
        let pred = env.as_contract(&client.address, || {
            use crate::storage_types::{DataKey, Prediction};
            env.storage()
                .persistent()
                .get::<DataKey, Prediction>(&DataKey::Prediction(market_id, predictor.clone()))
                .unwrap()
        });
        assert_eq!(pred.market_id, market_id);
        assert_eq!(pred.predictor, predictor);
        assert_eq!(pred.chosen_outcome, symbol_short!("yes"));
        assert_eq!(pred.stake_amount, 20_000_000);
        assert!(!pred.payout_claimed);
        assert_eq!(pred.payout_amount, 0);
    }

    // ── Validation: MarketNotFound ────────────────────────────────────────────

    #[test]
    fn submit_prediction_fails_market_not_found() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let predictor = Address::generate(&env);
        fund(&env, &xlm_token, &predictor, 20_000_000);

        let result = client.try_submit_prediction(
            &predictor,
            &99_u64,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );
        assert!(matches!(result, Err(Ok(InsightArenaError::MarketNotFound))));
    }

    // ── Validation: MarketExpired ─────────────────────────────────────────────

    #[test]
    fn submit_prediction_fails_market_expired() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);

        // Advance past end_time
        env.ledger().set_timestamp(env.ledger().timestamp() + 1001);

        let result = client.try_submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );
        assert!(matches!(result, Err(Ok(InsightArenaError::MarketExpired))));
    }

    // ── Validation: InvalidOutcome ────────────────────────────────────────────

    #[test]
    fn submit_prediction_fails_invalid_outcome() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);

        let result = client.try_submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("maybe"),
            &20_000_000_i128,
        );
        assert!(matches!(result, Err(Ok(InsightArenaError::InvalidOutcome))));
    }

    // ── Validation: StakeTooLow ───────────────────────────────────────────────

    #[test]
    fn submit_prediction_fails_stake_too_low() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);

        // min_stake is 10_000_000; submit 1 stroop below
        let result = client.try_submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &9_999_999_i128,
        );
        assert!(matches!(result, Err(Ok(InsightArenaError::StakeTooLow))));
    }

    // ── Validation: StakeTooHigh ──────────────────────────────────────────────

    #[test]
    fn submit_prediction_fails_stake_too_high() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 200_000_000);

        // max_stake is 100_000_000; submit 1 stroop above
        let result = client.try_submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &100_000_001_i128,
        );
        assert!(matches!(result, Err(Ok(InsightArenaError::StakeTooHigh))));
    }

    // ── Validation: AlreadyPredicted ──────────────────────────────────────────

    #[test]
    fn submit_prediction_fails_already_predicted() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 40_000_000);

        // First prediction succeeds
        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );

        // Second prediction for the same market must fail
        let result = client.try_submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("no"),
            &20_000_000_i128,
        );
        assert!(matches!(
            result,
            Err(Ok(InsightArenaError::AlreadyPredicted))
        ));
    }

    // ── Validation: Paused ────────────────────────────────────────────────────

    #[test]
    fn submit_prediction_fails_when_paused() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);

        client.set_paused(&true);

        let result = client.try_submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );
        assert!(matches!(result, Err(Ok(InsightArenaError::Paused))));
    }

    // ── XLM transfer: escrow receives the stake ───────────────────────────────

    #[test]
    fn submit_prediction_transfers_xlm_to_contract() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stake: i128 = 20_000_000;

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, stake);

        let token = TokenClient::new(&env, &xlm_token);
        assert_eq!(token.balance(&predictor), stake);
        assert_eq!(token.balance(&client.address), 0);

        client.submit_prediction(&predictor, &market_id, &symbol_short!("yes"), &stake);

        assert_eq!(token.balance(&predictor), 0);
        assert_eq!(token.balance(&client.address), stake);
    }

    // ── Market stats: total_pool and participant_count updated ────────────────

    #[test]
    fn submit_prediction_updates_market_total_pool_and_participant_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor_a = Address::generate(&env);
        let predictor_b = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor_a, 20_000_000);
        fund(&env, &xlm_token, &predictor_b, 30_000_000);

        client.submit_prediction(
            &predictor_a,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );
        client.submit_prediction(
            &predictor_b,
            &market_id,
            &symbol_short!("no"),
            &30_000_000_i128,
        );

        let market = client.get_market(&market_id);
        assert_eq!(market.total_pool, 50_000_000);
        assert_eq!(market.participant_count, 2);
    }

    // ── UserProfile: stats created and incremented correctly ─────────────────

    #[test]
    fn submit_prediction_creates_and_updates_user_profile() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stake: i128 = 20_000_000;

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, stake);

        client.submit_prediction(&predictor, &market_id, &symbol_short!("yes"), &stake);

        let profile = env.as_contract(&client.address, || {
            use crate::storage_types::{DataKey, UserProfile};
            env.storage()
                .persistent()
                .get::<DataKey, UserProfile>(&DataKey::User(predictor.clone()))
                .unwrap()
        });
        assert_eq!(profile.total_predictions, 1);
        assert_eq!(profile.total_staked, stake);
        assert_eq!(profile.address, predictor);
    }

    #[test]
    fn submit_prediction_accumulates_user_profile_across_markets() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        // Create two separate markets
        let market_id_1 = client.create_market(&creator, &default_params(&env));
        let market_id_2 = client.create_market(&creator, &default_params(&env));

        fund(&env, &xlm_token, &predictor, 50_000_000);

        client.submit_prediction(
            &predictor,
            &market_id_1,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );
        client.submit_prediction(
            &predictor,
            &market_id_2,
            &symbol_short!("no"),
            &30_000_000_i128,
        );

        let profile = env.as_contract(&client.address, || {
            use crate::storage_types::{DataKey, UserProfile};
            env.storage()
                .persistent()
                .get::<DataKey, UserProfile>(&DataKey::User(predictor.clone()))
                .unwrap()
        });
        assert_eq!(profile.total_predictions, 2);
        assert_eq!(profile.total_staked, 50_000_000);
    }

    // ── PredictorList: predictor appended correctly ───────────────────────────

    #[test]
    fn submit_prediction_appends_to_predictor_list() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor_a = Address::generate(&env);
        let predictor_b = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor_a, 20_000_000);
        fund(&env, &xlm_token, &predictor_b, 20_000_000);

        client.submit_prediction(
            &predictor_a,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );
        client.submit_prediction(
            &predictor_b,
            &market_id,
            &symbol_short!("no"),
            &20_000_000_i128,
        );

        let list = env.as_contract(&client.address, || {
            use crate::storage_types::DataKey;
            env.storage()
                .persistent()
                .get::<DataKey, soroban_sdk::Vec<Address>>(&DataKey::PredictorList(market_id))
                .unwrap()
        });
        assert_eq!(list.len(), 2);
        assert_eq!(list.get(0).unwrap(), predictor_a);
        assert_eq!(list.get(1).unwrap(), predictor_b);
    }

    // ── Boundary: exact min_stake and max_stake are accepted ─────────────────

    #[test]
    fn submit_prediction_accepts_exact_min_stake() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 10_000_000);

        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &10_000_000_i128,
        );
        let market = client.get_market(&market_id);
        assert_eq!(market.total_pool, 10_000_000);
    }

    #[test]
    fn submit_prediction_accepts_exact_max_stake() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 100_000_000);

        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &100_000_000_i128,
        );
        let market = client.get_market(&market_id);
        assert_eq!(market.total_pool, 100_000_000);
    }

    // ── get_prediction tests ──────────────────────────────────────────────────

    /// Returns the full Prediction struct when the record exists.
    #[test]
    fn get_prediction_returns_correct_struct() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stake: i128 = 20_000_000;

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, stake);
        client.submit_prediction(&predictor, &market_id, &symbol_short!("yes"), &stake);

        let pred = client.get_prediction(&market_id, &predictor);

        assert_eq!(pred.market_id, market_id);
        assert_eq!(pred.predictor, predictor);
        assert_eq!(pred.chosen_outcome, symbol_short!("yes"));
        assert_eq!(pred.stake_amount, stake);
        assert!(!pred.payout_claimed);
        assert_eq!(pred.payout_amount, 0);
    }

    /// Returns `PredictionNotFound` when no prediction exists for the key.
    #[test]
    fn get_prediction_returns_not_found_for_missing_key() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = deploy(&env);
        let predictor = Address::generate(&env);

        let result = client.try_get_prediction(&99_u64, &predictor);
        assert!(matches!(
            result,
            Err(Ok(InsightArenaError::PredictionNotFound))
        ));
    }

    /// `get_prediction` on a predictor address that has not staked on a real market
    /// also returns `PredictionNotFound`.
    #[test]
    fn get_prediction_returns_not_found_for_wrong_predictor() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stranger = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);
        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );

        // stranger never predicted — must get PredictionNotFound
        let result = client.try_get_prediction(&market_id, &stranger);
        assert!(matches!(
            result,
            Err(Ok(InsightArenaError::PredictionNotFound))
        ));
    }

    /// `get_prediction` does not mutate market state.
    #[test]
    fn get_prediction_does_not_mutate_market() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stake: i128 = 20_000_000;

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, stake);
        client.submit_prediction(&predictor, &market_id, &symbol_short!("yes"), &stake);

        let before = client.get_market(&market_id);
        client.get_prediction(&market_id, &predictor);
        let after = client.get_market(&market_id);

        assert_eq!(before.total_pool, after.total_pool);
        assert_eq!(before.participant_count, after.participant_count);
        assert_eq!(before.is_closed, after.is_closed);
    }

    /// `get_prediction` does not mutate the prediction record itself.
    #[test]
    fn get_prediction_does_not_mutate_prediction_record() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stake: i128 = 20_000_000;

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, stake);
        client.submit_prediction(&predictor, &market_id, &symbol_short!("yes"), &stake);

        let first = client.get_prediction(&market_id, &predictor);
        let second = client.get_prediction(&market_id, &predictor);

        assert_eq!(first, second);
    }

    /// Calling `get_prediction` multiple times always returns the same struct.
    #[test]
    fn get_prediction_is_idempotent() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stake: i128 = 50_000_000;

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, stake);
        client.submit_prediction(&predictor, &market_id, &symbol_short!("no"), &stake);

        for _ in 0..3 {
            let pred = client.get_prediction(&market_id, &predictor);
            assert_eq!(pred.stake_amount, stake);
            assert_eq!(pred.chosen_outcome, symbol_short!("no"));
        }
    }

    // ── has_predicted tests ───────────────────────────────────────────────

    #[test]
    fn has_predicted_returns_true_after_submission() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);
        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );

        assert!(client.has_predicted(&market_id, &predictor));
    }

    #[test]
    fn has_predicted_returns_false_when_not_predicted() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stranger = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);
        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );

        assert!(!client.has_predicted(&market_id, &stranger));
    }

    #[test]
    fn has_predicted_returns_false_for_nonexistent_market() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = deploy(&env);
        let predictor = Address::generate(&env);

        assert!(!client.has_predicted(&999_u64, &predictor));
    }

    #[test]
    fn has_predicted_never_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = deploy(&env);
        let random = Address::generate(&env);

        // No markets, no predictions — must return false, not panic
        let result = client.has_predicted(&0_u64, &random);
        assert!(!result);
    }

    #[test]
    fn has_predicted_does_not_mutate_state() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, 20_000_000);
        client.submit_prediction(
            &predictor,
            &market_id,
            &symbol_short!("yes"),
            &20_000_000_i128,
        );

        let market_before = client.get_market(&market_id);
        client.has_predicted(&market_id, &predictor);
        let market_after = client.get_market(&market_id);

        assert_eq!(market_before.total_pool, market_after.total_pool);
        assert_eq!(market_before.participant_count, market_after.participant_count);
    }

    // ── list_market_predictions tests ─────────────────────────────────────

    #[test]
    fn list_market_predictions_returns_all_predictions() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);
        let p3 = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &p1, 20_000_000);
        fund(&env, &xlm_token, &p2, 30_000_000);
        fund(&env, &xlm_token, &p3, 15_000_000);

        client.submit_prediction(&p1, &market_id, &symbol_short!("yes"), &20_000_000_i128);
        client.submit_prediction(&p2, &market_id, &symbol_short!("no"), &30_000_000_i128);
        client.submit_prediction(&p3, &market_id, &symbol_short!("yes"), &15_000_000_i128);

        let predictions = client.list_market_predictions(&market_id);
        assert_eq!(predictions.len(), 3);
        assert_eq!(predictions.get(0).unwrap().predictor, p1);
        assert_eq!(predictions.get(1).unwrap().predictor, p2);
        assert_eq!(predictions.get(2).unwrap().predictor, p3);
    }

    #[test]
    fn list_market_predictions_returns_empty_for_no_predictions() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = deploy(&env);
        let creator = Address::generate(&env);

        let market_id = client.create_market(&creator, &default_params(&env));
        let predictions = client.list_market_predictions(&market_id);
        assert_eq!(predictions.len(), 0);
    }

    #[test]
    fn list_market_predictions_returns_empty_for_nonexistent_market() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _) = deploy(&env);

        let predictions = client.list_market_predictions(&999_u64);
        assert_eq!(predictions.len(), 0);
    }

    #[test]
    fn list_market_predictions_contains_correct_data() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let predictor = Address::generate(&env);
        let stake: i128 = 25_000_000;

        let market_id = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &predictor, stake);
        client.submit_prediction(&predictor, &market_id, &symbol_short!("no"), &stake);

        let predictions = client.list_market_predictions(&market_id);
        assert_eq!(predictions.len(), 1);

        let pred = predictions.get(0).unwrap();
        assert_eq!(pred.market_id, market_id);
        assert_eq!(pred.predictor, predictor);
        assert_eq!(pred.chosen_outcome, symbol_short!("no"));
        assert_eq!(pred.stake_amount, stake);
        assert!(!pred.payout_claimed);
    }

    #[test]
    fn list_market_predictions_isolated_per_market() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, xlm_token) = deploy(&env);
        let creator = Address::generate(&env);
        let p1 = Address::generate(&env);
        let p2 = Address::generate(&env);

        let m1 = client.create_market(&creator, &default_params(&env));
        let m2 = client.create_market(&creator, &default_params(&env));
        fund(&env, &xlm_token, &p1, 40_000_000);
        fund(&env, &xlm_token, &p2, 20_000_000);

        client.submit_prediction(&p1, &m1, &symbol_short!("yes"), &20_000_000_i128);
        client.submit_prediction(&p1, &m2, &symbol_short!("no"), &20_000_000_i128);
        client.submit_prediction(&p2, &m1, &symbol_short!("no"), &20_000_000_i128);

        let m1_preds = client.list_market_predictions(&m1);
        let m2_preds = client.list_market_predictions(&m2);

        assert_eq!(m1_preds.len(), 2);
        assert_eq!(m2_preds.len(), 1);
    }
}
