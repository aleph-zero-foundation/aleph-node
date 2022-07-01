use std::{
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    thread::JoinHandle,
};

use aleph_client::FromStr;
use log::info;
use serde_json::Value;
use sp_core::H256 as Hash;
use substrate_api_client::{
    rpc::{
        json_req,
        ws_client::{
            on_extrinsic_msg_submit_only, on_extrinsic_msg_until_broadcast,
            on_extrinsic_msg_until_finalized, on_extrinsic_msg_until_in_block,
            on_extrinsic_msg_until_ready, on_get_request_msg, OnMessageFn, RpcClient,
        },
    },
    ApiClientError, ApiResult, FromHexString, RpcClient as RpcClientTrait, XtStatus,
};
use ws::{
    connect, Error as WsError, ErrorKind, Handler, Message, Result as WsResult, Sender as WsSender,
};

// It attempts to run a single thread with a single WebSocket connection that processes all outgoing requests.
// The upstream approach is to open a new socket for every request, but the number of sockets ends
// up being the bottleneck for flooding, so we need to do it better.
pub struct WsRpcClient {
    mux: Mutex<()>,
    next_handler: Arc<Mutex<Option<RpcClient>>>,
    join_handle: Option<thread::JoinHandle<WsResult<()>>>,
    out: WsSender,
    url: String,
}

impl WsRpcClient {
    pub fn new(url: &str) -> Result<WsRpcClient, String> {
        let RunningRpcClient {
            ws_sender,
            client_handle,
            client,
        } = start_rpc_client_thread(url.to_string())?;

        Ok(WsRpcClient {
            next_handler: client,
            join_handle: Some(client_handle),
            out: ws_sender,
            mux: Mutex::new(()),
            url: url.to_string(),
        })
    }
}

impl Clone for WsRpcClient {
    fn clone(&self) -> Self {
        Self::new(&self.url).unwrap()
    }
}

impl Drop for WsRpcClient {
    fn drop(&mut self) {
        self.close();
    }
}

impl RpcClientTrait for WsRpcClient {
    fn get_request(&self, jsonreq: Value) -> ApiResult<String> {
        let _mux = self.mux.lock();

        Ok(self.get(jsonreq.to_string())?)
    }

    fn send_extrinsic(
        &self,
        xthex_prefixed: String,
        exit_on: XtStatus,
    ) -> ApiResult<Option<sp_core::H256>> {
        let _mux = self.mux.lock();

        let jsonreq = match exit_on {
            XtStatus::SubmitOnly => json_req::author_submit_extrinsic(&xthex_prefixed).to_string(),
            _ => json_req::author_submit_and_watch_extrinsic(&xthex_prefixed).to_string(),
        };

        let result = match exit_on {
            XtStatus::Finalized => {
                let res = self.send_extrinsic_and_wait_until_finalized(jsonreq)?;
                info!("finalized: {}", res);
                Ok(Some(Hash::from_hex(res)?))
            }
            XtStatus::InBlock => {
                let res = self.send_extrinsic_and_wait_until_in_block(jsonreq)?;
                info!("inBlock: {}", res);
                Ok(Some(Hash::from_hex(res)?))
            }
            XtStatus::Broadcast => {
                let res = self.send_extrinsic_and_wait_until_broadcast(jsonreq)?;
                info!("broadcast: {}", res);
                Ok(None)
            }
            XtStatus::Ready => {
                let res = self.send_extrinsic_until_ready(jsonreq)?;
                info!("ready: {}", res);
                Ok(None)
            }
            XtStatus::SubmitOnly => {
                let res = self.send_extrinsic(jsonreq)?;
                info!("submitted xt: {}", res);
                Ok(None)
            }
            _ => Err(ApiClientError::UnsupportedXtStatus(exit_on)),
        };

        self.set_next_handler(None);
        result
    }
}

impl FromStr for WsRpcClient {
    type Err = String;

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        WsRpcClient::new(url)
    }
}

impl WsRpcClient {
    fn get(&self, json_req: String) -> WsResult<String> {
        self.send_rpc_request(json_req, on_get_request_msg)
    }

    fn send_extrinsic(&self, json_req: String) -> WsResult<String> {
        self.send_rpc_request(json_req, on_extrinsic_msg_submit_only)
    }

    fn send_extrinsic_until_ready(&self, json_req: String) -> WsResult<String> {
        self.send_rpc_request(json_req, on_extrinsic_msg_until_ready)
    }

    fn send_extrinsic_and_wait_until_broadcast(&self, json_req: String) -> WsResult<String> {
        self.send_rpc_request(json_req, on_extrinsic_msg_until_broadcast)
    }

    fn send_extrinsic_and_wait_until_in_block(&self, json_req: String) -> WsResult<String> {
        self.send_rpc_request(json_req, on_extrinsic_msg_until_in_block)
    }

    fn send_extrinsic_and_wait_until_finalized(&self, json_req: String) -> WsResult<String> {
        self.send_rpc_request(json_req, on_extrinsic_msg_until_finalized)
    }

    // FIXME:
    // here we are using a deprecated `mio::channel::sync_channel` instead of the
    // now-recommended `mio-extras` channel, because the ws library (in the latest version)
    // is not yet updated to use the `mio-extras` version (and no conversion exists).
    // We need to change it to `mio-extras` as soon as `ws` is updated.
    #[allow(deprecated)]
    fn send_rpc_request(&self, jsonreq: String, on_message_fn: OnMessageFn) -> WsResult<String> {
        // ws_sender below is used by the RpcClient while being executed by another thread,
        // but we don't want it actually to do anything, since we are sending the given request here
        // 1 used by `on_open` of RpcClient + 1 for `close`
        const MAGIC_SEND_CONST: usize = 2;
        let (ws_tx, _ws_rx) = mio::channel::sync_channel(MAGIC_SEND_CONST);
        let ws_sender = ws::Sender::new(0.into(), ws_tx, 0);

        let (result_in, result_out) = channel();
        let rpc_client = RpcClient {
            out: ws_sender,
            request: jsonreq.clone(),
            result: result_in,
            on_message_fn,
        };
        self.set_next_handler(Some(rpc_client));
        self.out.send(jsonreq)?;
        let res = result_out.recv().map_err(|err| {
            WsError::new(
                ErrorKind::Custom(Box::new(err)),
                "unable to read an answer from the `result_out` channel",
            )
        })?;
        self.set_next_handler(None);
        WsResult::Ok(res)
    }

    pub fn close(&mut self) {
        self.out
            .shutdown()
            .expect("unable to send close on the WebSocket");
        self.join_handle
            .take()
            .map(|handle| handle.join().expect("unable to join WebSocket's thread"));
    }

    fn set_next_handler(&self, handler: Option<RpcClient>) {
        *self
            .next_handler
            .lock()
            .expect("unable to acquire a lock on RpcClient") = handler;
    }
}

struct RunningRpcClient {
    ws_sender: WsSender,
    client_handle: JoinHandle<WsResult<()>>,
    client: Arc<Mutex<Option<RpcClient>>>,
}

fn start_rpc_client_thread(url: String) -> Result<RunningRpcClient, String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    let rpc_client = Arc::new(Mutex::new(None));
    let connect_rpc_client = Arc::clone(&rpc_client);
    let join = thread::Builder::new()
        .name("client".to_owned())
        .spawn(|| -> WsResult<()> {
            connect(url, move |out| {
                tx.send(out).expect("main thread was already stopped");
                WsHandler {
                    next_handler: connect_rpc_client.clone(),
                }
            })
        })
        .map_err(|_| "unable to spawn WebSocket's thread")?;
    let out = rx.recv().map_err(|_| "WebSocket's unexpectedly died")?;
    Ok(RunningRpcClient {
        ws_sender: out,
        client_handle: join,
        client: rpc_client,
    })
}

struct WsHandler {
    next_handler: Arc<Mutex<Option<RpcClient>>>,
}

impl Handler for WsHandler {
    fn on_message(&mut self, msg: Message) -> WsResult<()> {
        if let Some(handler) = self
            .next_handler
            .lock()
            .expect("main thread probably died")
            .as_mut()
        {
            handler.on_message(msg)
        } else {
            Ok(())
        }
    }
}
