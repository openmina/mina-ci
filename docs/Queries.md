# Used GraphQL queries

## Queries to the node's regular GraphQL server

- {"query": "{ bestChain { stateHash protocolState { consensusState { blockHeight } } } }" }
- {"query": "{ daemonStatus { addrsAndPorts { externalIp, peer { peerId } } syncStatus metrics { transactionPoolSize transactionsAddedToPool transactionPoolDiffReceived transactionPoolDiffBroadcasted } } snarkPool { prover } }" }

## Queries to the internal trace consumer

- {"query": "{ blockStructuredTrace(block_identifier: \"{STATE_HASH}\" ) }" }
- {"query": "{ blockTraces(maxLength: 50, order: Descending) }" }

