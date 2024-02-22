use {
    super::{handle_submessages, new_after_tx_event, new_before_tx_event},
    crate::{AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{BlockInfo, Context, Event, Storage, Tx},
    cw_vm::Instance,
    tracing::{debug, warn},
};

// --------------------------------- before tx ---------------------------------

pub fn before_tx<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> AppResult<Vec<Event>> {
    match _before_tx(store, block, tx) {
        Ok(events) => {
            // TODO: add txhash here?
            debug!(sender = tx.sender.to_string(), "Before transaction hook called");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call before transaction hook");
            Err(err)
        },
    }
}

fn _before_tx<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &tx.sender)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &tx.sender]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call `before_tx` entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        tx.sender.clone(),
        sender:          None,
        funds:           None,
        simulate:        Some(false),
        submsg_result:   None,
    };
    let resp = instance.call_before_tx(&ctx, tx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_before_tx_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}

// --------------------------------- after tx ----------------------------------

pub fn after_tx<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> AppResult<Vec<Event>> {
    match _after_tx(store, block, tx) {
        Ok(events) => {
            // TODO: add txhash here?
            debug!(sender = tx.sender.to_string(), "After transaction hook called");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call after transaction hook");
            Err(err)
        },
    }
}

fn _after_tx<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &tx.sender)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &tx.sender]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call `after_tx` entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        tx.sender.clone(),
        sender:          None,
        funds:           None,
        simulate:        Some(false),
        submsg_result:   None,
    };
    let resp = instance.call_after_tx(&ctx, tx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_after_tx_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}