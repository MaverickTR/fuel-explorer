extern crate alloc;
use fuel_indexer_macros::indexer;
use fuel_indexer_plugin::prelude::*;
use std::collections::HashSet;

#[indexer(manifest = "indexer.manifest.yaml")]
pub mod indexer_index_mod {
    fn indexer_handler(block_data: BlockData) {
        let mut block_gas_limit = 0;

        // Convert the deserialized block `BlockData` struct that we get from our Fuel node, into
        // a block entity `Block` that we can persist to the database. The `Block` type below is
        // defined in our schema/explorer.graphql and represents the type that we will
        // save to our database.
        let producer = block_data.producer.unwrap_or(Bytes32::zeroed());

        let block = Block {
            id: first8_bytes_to_u64(block_data.id),
            height: block_data.height,
            producer,
            hash: block_data.id,
            hash4: block_data.id,
            timestamp: block_data.time,
            gas_limit: block_gas_limit,
        };

        // Now that we've created the object for the database, let's save it.
        block.save();

        // Keep track of some Receipt data involved in this transaction.
        let mut accounts = HashSet::new();
        let mut contracts = HashSet::new();

        for tx in block_data.transactions.iter() {
            let mut tx_amount = 0;
            let mut tokens_transferred = Vec::new();

            // `Transaction::Script`, `Transaction::Create`, and `Transaction::Mint`
            // are unused but demonstrate properties like gas, inputs,
            // outputs, script_data, and other pieces of metadata. You can access
            // properties that have the corresponding transaction `Field` traits
            // implemented; examples below.
            match &tx.transaction {
                #[allow(unused)]
                Transaction::Script(t) => {
                    Logger::info("Inside a script transaction. (>^‿^)>");

                    let gas_limit = t.gas_limit();
                    let gas_price = t.gas_price();
                    let maturity = t.maturity();
                    let script = t.script();
                    let script_data = t.script_data();
                    let receipts_root = t.receipts_root();
                    let inputs = t.inputs();
                    let outputs = t.outputs();
                    let witnesses = t.witnesses();

                    let json = &tx.transaction.to_json();
                    block_gas_limit += gas_limit;
                }
                #[allow(unused)]
                Transaction::Create(t) => {
                    Logger::info("Inside a create transaction. <(^.^)>");

                    let gas_limit = t.gas_limit();
                    let gas_price = t.gas_price();
                    let maturity = t.maturity();
                    let salt = t.salt();
                    let bytecode_length = t.bytecode_length();
                    let bytecode_witness_index = t.bytecode_witness_index();
                    let inputs = t.inputs();
                    let outputs = t.outputs();
                    let witnesses = t.witnesses();
                    let storage_slots = t.storage_slots();
                    block_gas_limit += gas_limit;
                }
                #[allow(unused)]
                Transaction::Mint(t) => {
                    Logger::info("Inside a mint transaction. <(^‿^<)");

                    let tx_pointer = t.tx_pointer();
                    let outputs = t.outputs();
                }
            }

            for receipt in &tx.receipts {
                // You can handle each receipt in a transaction `TransactionData` as you like.
                //
                // Below demonstrates how you can use parts of a receipt `Receipt` in order
                // to persist entities defined in your GraphQL schema, to the database.
                match receipt {
                    #[allow(unused)]
                    Receipt::Call { id, .. } => {
                        contracts.insert(Contract {
                            id: *id,
                            last_seen: 0,
                        });
                    }
                    #[allow(unused)]
                    Receipt::ReturnData { id, .. } => {
                        contracts.insert(Contract {
                            id: *id,
                            last_seen: 0,
                        });
                    }
                    #[allow(unused)]
                    Receipt::Transfer {
                        id,
                        to,
                        asset_id,
                        amount,
                        ..
                    } => {
                        contracts.insert(Contract {
                            id: *id,
                            last_seen: 0,
                        });

                        let transfer = Transfer {
                            id: first8_bytes_to_u64(bytes32_from_inputs(
                                id,
                                [id.to_vec(), to.to_vec(), asset_id.to_vec()].concat(),
                            )),
                            contract_id: *id,
                            receiver: *to,
                            amount: *amount,
                            asset_id: *asset_id,
                        };

                        transfer.save();
                        tokens_transferred.push(asset_id.to_string());
                    }
                    #[allow(unused)]
                    Receipt::TransferOut {
                        id,
                        to,
                        amount,
                        asset_id,
                        ..
                    } => {
                        contracts.insert(Contract {
                            id: *id,
                            last_seen: 0,
                        });

                        accounts.insert(Account {
                            id: *to,
                            last_seen: 0,
                        });

                        tx_amount += amount;
                        let transfer_out = TransferOut {
                            id: first8_bytes_to_u64(bytes32_from_inputs(
                                id,
                                [id.to_vec(), to.to_vec(), asset_id.to_vec()].concat(),
                            )),
                            contract_id: *id,
                            receiver: *to,
                            amount: *amount,
                            asset_id: *asset_id,
                        };

                        transfer_out.save();
                    }
                    #[allow(unused)]
                    Receipt::Log { id, rb, .. } => {
                        contracts.insert(Contract {
                            id: *id,
                            last_seen: 0,
                        });
                        let log = Log {
                            id: first8_bytes_to_u64(bytes32_from_inputs(
                                id,
                                u64::to_le_bytes(*rb).to_vec(),
                            )),
                            contract_id: *id,
                            rb: *rb,
                        };

                        log.save();
                    }
                    #[allow(unused)]
                    Receipt::LogData { id, .. } => {
                        contracts.insert(Contract {
                            id: *id,
                            last_seen: 0,
                        });

                        Logger::info("LogData types are unused in this example. (>'')>");
                    }
                    #[allow(unused)]
                    Receipt::ScriptResult { result, gas_used } => {
                        let result: u64 = match result {
                            ScriptExecutionResult::Success => 1,
                            ScriptExecutionResult::Revert => 2,
                            ScriptExecutionResult::Panic => 3,
                            ScriptExecutionResult::GenericFailure(_) => 4,
                        };
                        let r = ScriptResult {
                            id: first8_bytes_to_u64(bytes32_from_inputs(
                                &[0u8; 32],
                                u64::to_be_bytes(result).to_vec(),
                            )),
                            result,
                            gas_used: *gas_used,
                        };
                        r.save();
                    }
                    #[allow(unused)]
                    Receipt::MessageOut {
                        sender,
                        recipient,
                        amount,
                        ..
                    } => {
                        tx_amount += amount;
                        accounts.insert(Account {
                            id: *sender,
                            last_seen: 0,
                        });
                        accounts.insert(Account {
                            id: *recipient,
                            last_seen: 0,
                        });

                        Logger::info("LogData types are unused in this example. (>'')>");
                    }
                    _ => {
                        Logger::info("This type is not handled yet.");
                    }
                }
            }

            // Persist the transaction to the database via the `Tx` object defined in the GraphQL schema.
            let tx_entity = Tx {
                block: block.id,
                hash: tx.id,
                timestamp: block.timestamp,
                id: first8_bytes_to_u64(tx.id),
                value: tx_amount,
                status: tx.status.clone().into(),
                tokens_transferred: Json(
                    serde_json::to_value(tokens_transferred)
                        .unwrap()
                        .to_string(),
                ),
            };

            tx_entity.save();
        }

        // Save all of our accounts
        for account in accounts.iter() {
            account.save();
        }

        // Save all of our contracts
        for contract in contracts.iter() {
            contract.save();
        }
    }
}
