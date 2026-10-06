// Says so when this page opens a WebRTC connection (#839).
//
// The browser refuses the traffic itself (`browser.rs`'s flags), so this is
// not the wall, only the word: a WebSocket is reported by the protocol, a peer
// connection by nothing, and a page that wanted the network should hear it did
// not get it. The binding is the capture's (`visitor.rs`); a page opened any
// other way, with no binding, gets the constructors untouched.
(() => {
  const say = window.__scorsese_offline;
  if (typeof say !== "function") return;
  for (const name of ["RTCPeerConnection", "webkitRTCPeerConnection"]) {
    const Real = window[name];
    if (typeof Real !== "function") continue;
    window[name] = class extends Real {
      constructor(...args) {
        say("WebRTC");
        super(...args);
      }
    };
  }
})();
