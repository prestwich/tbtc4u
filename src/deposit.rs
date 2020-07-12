use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use futures_timer::Delay;
use futures_util::{
    future::Join,
    stream::{self, Stream, StreamExt},
    FutureExt,
};
use pin_project::pin_project;

use ethers::providers::{
    JsonRpcClient, PendingTransaction as EthPendingTx, Provider, ProviderError as EthProviderError,
};
use ethers_core::types::{H256, U256};

use rmn_btc::prelude::*;
use rmn_btc_provider::{PollingBTCProvider, ProviderError as BTCProviderError};

type ProviderFut<'a, T, E> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + 'a + Send>>;

type BTCFut<'a, T> = ProviderFut<'a, T, BTCProviderError>;
type EthFut<'a, T> = ProviderFut<'a, T, EthProviderError>;

// Used to unpause things blocked by an interval. Uses `ready!` to shortcut to Pending
// if the interval has not yet elapsed
macro_rules! unpause {
    ($ctx:expr, $interval:expr, $next_fut:expr) => {{
        let _ready = futures_util::ready!($interval.poll_next_unpin($ctx));
        $ctx.waker().wake_by_ref();
        Box::pin($next_fut)
    }};
}

/// Async delay stream
pub(crate) fn new_interval(duration: Duration) -> impl Stream<Item = ()> + Send + Unpin {
    stream::unfold((), move |_| Delay::new(duration).map(|_| Some(((), ())))).map(drop)
}

pub enum DepositStates<'a, P: JsonRpcClient> {
    Updating(Pin<Box<Join<EthFut<'a, U256>, EthFut<'a, U256>>>>),
    PausedPollingState,
    PollingState(EthFut<'a, U256>),
    AwaitingFund(BTCFut<'a, Vec<UTXO>>),
    PausedGettingProof(TXID),
    GettingProof(TXID, BTCFut<'a, Option<Vec<TXID>>>),
    SubmittingProof(EthFut<'a, H256>),
    TrackingProofTx(EthPendingTx<'a, P>),
    Complete,
    Failed,
}

impl<P: JsonRpcClient> std::fmt::Debug for DepositStates<'_, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DepositStates::Updating(_) => "GettingState",
            DepositStates::PausedPollingState => "PausedPollingState",
            DepositStates::PollingState(_) => "PollingState",
            DepositStates::AwaitingFund(_) => "AwaitingFund",
            DepositStates::PausedGettingProof(_) => "PausedGettingProof",
            DepositStates::GettingProof(_, _) => "GettingProof",
            DepositStates::SubmittingProof(_) => "SubmittingProof",
            DepositStates::TrackingProofTx(_) => "TrackingProofTx",
            DepositStates::Complete => "Complete",
            DepositStates::Failed => "Failed :(",
        };
        f.write_str(s)
    }
}

#[pin_project(project = DepositProj)]
pub struct Deposit<'a, P: JsonRpcClient> {
    // address: Address,
    sats_expected: Option<u64>,
    // contract: DepositContract,
    state: DepositStates<'a, P>,
    interval: Box<dyn Stream<Item = ()> + Send + Unpin>,
    ether: &'a Provider<P>,
    bitcoin: &'a dyn PollingBTCProvider,
}

impl<'a, P: JsonRpcClient> std::fmt::Debug for Deposit<'a, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Deposit")
            // .field("address", &self.address)
            .field("sats_expected", &self.sats_expected)
            .field("state", &self.state)
            .finish()
    }
}

impl<'a, P: JsonRpcClient> Deposit<'a, P> {
    pub fn new(
        // contract: &'static Contract,
        ether: &'a Provider<P>,
        bitcoin: &'a dyn PollingBTCProvider,
    ) -> Self {
        // let fut = provider.call(req, None);
        Self {
            state: DepositStates::Failed,
            sats_expected: None,
            // state: DepositStates::Updating(futures_util::join(fut1, fut2)),
            // contract,
            interval: Box::new(new_interval(std::time::Duration::from_secs(15))),
            ether,
            bitcoin,
        }
    }
}

impl<'a, P: JsonRpcClient> std::future::Future for Deposit<'a, P> {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<bool> {
        let DepositProj {
            // address,
            sats_expected,
            state,
            // contract,
            interval,
            ether: _ether,
            bitcoin,
        } = self.project();

        match state {
            DepositStates::Updating(fut) => {
                if let (Ok(current_state), Ok(sats)) = futures_util::ready!(fut.as_mut().poll(ctx))
                {
                    if current_state.low_u64() > 3 {
                        *state = DepositStates::Complete;
                        return Poll::Ready(true);
                    }
                    *sats_expected = Some(sats.low_u64());

                // // TODO:
                // let fut = Box::pin(contract.get_current_state());
                // *state = DepositStates::PollingState(fut)
                } else {
                    *state = DepositStates::Failed;
                    return Poll::Ready(false);
                }
            }
            DepositStates::PausedPollingState => {
                // // TODO:
                // let fut = unpause!(ctx, interval, contract.get_current_state());
                // *state = DepositStates::PollingState(fut);
            }
            DepositStates::PollingState(fut) => {
                if let Ok(new_deposit_state) = futures_util::ready!(fut.as_mut().poll(ctx)) {
                    if new_deposit_state.low_u64() == 2 {
                        // // TODO:
                        // let fut = bitcoin.find_utxos_by_script();
                        // *state = DepositStates::AwaitingFund(fut);
                    }
                    if new_deposit_state.low_u64() > 3 {
                        *state = DepositStates::Complete;
                        return Poll::Ready(true);
                    }
                } else {
                    *state = DepositStates::Failed;
                    return Poll::Ready(false);
                }
            }
            DepositStates::AwaitingFund(fut) => {
                if let Ok(utxos) = futures_util::ready!(fut.as_mut().poll(ctx)) {
                    if let Some(fund_txo) = utxos
                        .into_iter()
                        .find(|u| &u.value >= sats_expected.as_ref().unwrap())
                    {
                        // // TODO:
                        let txid = fund_txo.outpoint.txid;
                        let fut = Box::pin(bitcoin.get_merkle(txid));
                        *state = DepositStates::GettingProof(txid, fut);
                    }
                } else {
                    *state = DepositStates::Failed;
                    return Poll::Ready(false);
                }
            }
            DepositStates::PausedGettingProof(txid) => {
                // TODO:
                let fut = unpause!(ctx, interval, bitcoin.get_merkle(*txid));
                *state = DepositStates::GettingProof(*txid, fut);
            }
            DepositStates::GettingProof(_txid, _fut) => {

            }
            DepositStates::SubmittingProof(_fut) => {}
            DepositStates::TrackingProofTx(_ptx) => {}
            DepositStates::Complete => {
                panic!("polled after completion")
            }
            DepositStates::Failed => {
                panic!("polled after completion")
            }
        }

        Poll::Pending
    }
}
