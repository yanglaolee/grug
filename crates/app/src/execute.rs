use {
    crate::{Querier, ACCOUNTS, CODES, CONFIG, CONTRACT_NAMESPACE},
    anyhow::ensure,
    cw_db::PrefixStore,
    cw_std::{
        hash, Account, Addr, Binary, BlockInfo, Coins, Context, Hash, Message, Storage, TransferMsg,
    },
    cw_vm::Instance,
    tracing::{info, warn},
};

pub fn process_msg<S: Storage + Clone + 'static>(
    mut store: S,
    block:     &BlockInfo,
    sender:    &Addr,
    msg:       Message,
) -> anyhow::Result<()> {
    match msg {
        Message::Transfer {
            to,
            coins,
        } => transfer(store, block, sender.clone(), to, coins),
        Message::StoreCode {
            wasm_byte_code,
        } => store_code(&mut store, &wasm_byte_code),
        Message::Instantiate {
            code_hash,
            msg,
            salt,
            funds,
            admin,
        } => instantiate(store, block, sender, code_hash, msg, salt, funds, admin),
        Message::Execute {
            contract,
            msg,
            funds,
        } => execute(store, block, sender, &contract, msg, funds),
    }
}

// -------------------------------- store code ---------------------------------

fn store_code(store: &mut dyn Storage, wasm_byte_code: &Binary) -> anyhow::Result<()> {
    match _store_code(store, wasm_byte_code) {
        Ok(code_hash) => {
            info!(code_hash = code_hash.to_string(), "stored code");
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "failed to store code");
            Err(err)
        },
    }
}

// return the hash of the code that is stored.
fn _store_code(store: &mut dyn Storage, wasm_byte_code: &Binary) -> anyhow::Result<Hash> {
    // TODO: static check, ensure wasm code has necessary imports/exports
    let code_hash = hash(wasm_byte_code);

    ensure!(!CODES.has(store, &code_hash), "code with hash `{code_hash}` already exists");

    CODES.save(store, &code_hash, wasm_byte_code)?;

    Ok(code_hash)
}

// --------------------------------- transfer ----------------------------------

fn transfer<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    from:  Addr,
    to:    Addr,
    coins: Coins,
) -> anyhow::Result<()> {
    match _transfer(store, block, from, to, coins) {
        Ok(TransferMsg { from, to, coins }) => {
            info!(
                from  = from.to_string(),
                to    = to.to_string(),
                coins = coins.to_string(),
                "transferred coins"
            );
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "failed to transfer coins");
            Err(err)
        },
    }
}

fn _transfer<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    from:  Addr,
    to:    Addr,
    coins: Coins,
) -> anyhow::Result<TransferMsg> {
    // load wasm code
    let cfg = CONFIG.load(&store)?;
    let account = ACCOUNTS.load(&store, &cfg.bank)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &cfg.bank]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call transfer
    let ctx = Context {
        block:    block.clone(),
        contract: cfg.bank,
        sender:   None,
        simulate: None,
    };
    let msg = TransferMsg {
        from,
        to,
        coins,
    };
    let resp = instance.call_transfer(&ctx, &msg)?.into_std_result()?;

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(msg)
}

// -------------------------------- instantiate --------------------------------

#[allow(clippy::too_many_arguments)]
fn instantiate<S: Storage + Clone + 'static>(
    store:     S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> anyhow::Result<()> {
    match _instantiate(store, block, sender, code_hash, msg, salt, funds, admin) {
        Ok(address) => {
            info!(address = address.to_string(), "instantiated contract");
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "failed to instantiate contract");
            Err(err)
        },
    }
}

// return the address of the contract that is instantiated.
#[allow(clippy::too_many_arguments)]
fn _instantiate<S: Storage + Clone + 'static>(
    mut store: S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> anyhow::Result<Addr> {
    // load wasm code
    let wasm_byte_code = CODES.load(&store, &code_hash)?;

    // compute contract address and save account info
    let address = Addr::compute(sender, &code_hash, &salt);

    // there can't already be an account of the same address
    ACCOUNTS.update(&mut store, &address, |maybe_acct| {
        ensure!(maybe_acct.is_none(), "account with the address `{address}` already exists");
        Ok(Some(Account { code_hash, admin }))
    })?;

    // make the coin transfers
    if !funds.is_empty() {
        _transfer(store.clone(), block, sender.clone(), address.clone(), funds)?;
    }

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &address]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call instantiate
    let ctx = Context {
        block:    block.clone(),
        contract: address,
        sender:   Some(sender.clone()),
        simulate: None,
    };
    let resp = instance.call_instantiate(&ctx, msg)?.into_std_result()?;

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(ctx.contract)
}

// ---------------------------------- execute ----------------------------------

fn execute<S: Storage + Clone + 'static>(
    store:     S,
    block:     &BlockInfo,
    contract:  &Addr,
    sender:    &Addr,
    msg:       Binary,
    funds:     Coins,
) -> anyhow::Result<()> {
    match _execute(store, block, sender, contract, msg, funds) {
        Ok(()) => {
            info!(contract = contract.to_string(), "executed contract");
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "failed to execute contract");
            Err(err)
        },
    }
}

fn _execute<S: Storage + Clone + 'static>(
    store:     S,
    block:     &BlockInfo,
    contract:  &Addr,
    sender:    &Addr,
    msg:       Binary,
    funds:     Coins,
) -> anyhow::Result<()> {
    // make the coin transfers
    if !funds.is_empty() {
        _transfer(store.clone(), block, sender.clone(), contract.clone(), funds)?;
    }

    // load wasm code
    let account = ACCOUNTS.load(&store, contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call execute
    let ctx = Context {
        block:    block.clone(),
        contract: contract.clone(),
        sender:   Some(sender.clone()),
        simulate: None,
    };
    let resp = instance.call_execute(&ctx, msg)?.into_std_result()?;

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(())
}
