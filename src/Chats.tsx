import { useEffect, useRef, useState } from "react";
import { call } from "./bridge";
import { BubbleClose } from "./Explanation";
import type { Snapshot } from "./domain";
type Chat = { id:string; name?:string; preview:string; updatedAt:number; status?:{type:string; activeFlags?:string[]} };
type ChatPage = {data:Chat[];nextCursor?:string;live:boolean};
export default function Chats() {
  const [chats,setChats]=useState<Chat[]>([]),[selected,setSelected]=useState<Chat|null>(null);
  const [live,setLive]=useState(false),[cursor,setCursor]=useState<string>(),[busy,setBusy]=useState(false),[error,setError]=useState(""),[message,setMessage]=useState(""),[sent,setSent]=useState("");
  const alive=useRef(true);
  async function load(more=false) {
    setBusy(true);setError("");
    try {const result=await call<ChatPage>("kitty_chats",{cursor:more?cursor:null});if(!alive.current)return;
      setChats(old=>more?[...old,...result.data.filter(c=>!old.some(o=>o.id===c.id))]:result.data);
      setCursor(result.nextCursor || undefined);setLive(result.live);
      setSelected(old=>old?result.data.find(c=>c.id===old.id)||old:null);
    } catch(e){if(alive.current)setError(String(e));}finally{if(alive.current)setBusy(false);}
  }
  useEffect(()=>{alive.current=true;void call<Snapshot>("snapshot").then(s=>document.documentElement.dataset.theme=s.theme).catch(()=>{});void load();return()=>{alive.current=false;};},[]);
  const canSend=live && !!selected && ["idle","active"].includes(selected.status?.type || "");
  return <main className="explanation-bubble">
    <BubbleClose/><header><h1>Codex chats</h1></header>
    <p className="bubble-source">{live?"Live Codex connection":"Recent local history · live status unavailable"}</p>
    <button className="small-button" disabled={busy} onClick={()=>void load()}>Refresh chats</button>
    {busy && <p role="status"><span className="kitty-thinking" aria-hidden="true"><i/><i/><i/></span> Loading chats…</p>}
    {error && <p role="alert">{error}</p>}
    <div className="chat-list" aria-label="Recent Codex chats">{chats.map(chat=><button key={chat.id} className="chat-row" aria-pressed={selected?.id===chat.id} onClick={()=>{setSelected(chat);setSent("");}}>
      <strong>{chat.name || chat.preview || "Untitled chat"}</strong><small><span className="chat-state" data-state={live?chat.status?.type:"unknown"}>{live && chat.status?.type==="active"?"Running":live && chat.status?.type==="idle"?"Idle":"Status unavailable"}</span> · {new Date(chat.updatedAt*1000).toLocaleDateString()}</small>
    </button>)}</div>
    {!busy && !chats.length && !error && <p>No recent local Codex chats found.</p>}
    {cursor && <button className="small-button" disabled={busy} onClick={()=>void load(true)}>Load older chats</button>}
    {selected && <form className="chat-compose" onSubmit={async e=>{e.preventDefault();if(!canSend || busy)return;setBusy(true);setError("");setSent("");try{await call("kitty_chat_send",{threadId:selected.id,message});setMessage("");setSent("Message accepted by Codex.");}catch(e){setError(String(e));}finally{setBusy(false);}}}>
      <label htmlFor="chat-message">Message to {selected.name || "selected chat"}</label>
      <textarea id="chat-message" value={message} maxLength={8192} onChange={e=>setMessage(e.target.value)} placeholder="Send a follow-up…" />
      <button className="small-button" disabled={!canSend || busy || !message.trim()}>{selected.status?.type==="active" && live?"Send to running task":"Send message"}</button>
      {!canSend && <p className="bubble-source">Continue this chat in Codex. Kitty cannot send safely without its live connection.</p>}
    </form>}
    <p role="status">{sent}</p>
  </main>;
}
