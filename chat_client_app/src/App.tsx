import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import "bootstrap/dist/css/bootstrap.min.css";
import "./App.css";
import Message from "./components/Message";

interface ConnectionStatus {
  message: string;
  code: number;
}

interface ChatMessage {
  user_id: string;
  timestamp: string;
  content: string;
}

const App = () => {
  const [msg_out, setMsg_out] = useState<string>("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [connStat, setConnStat] = useState<ConnectionStatus>({
    message: "undefined",
    code: 500,
  });

  useEffect(() => {
    const unmountConnStat = listen<number>("conn_stat", (event) => {
      const status: number = event.payload;
      console.log({ status: status });
      switch (status) {
        case 200:
          setConnStat({
            message: "Connection Succesfully Established",
            code: status,
          });
          break;
        case 404:
          setConnStat({
            message: "Server is offline of Unreachable",
            code: status,
          });
          break;
        default:
          setConnStat({
            message: "Unknown server error",
            code: 500,
          });
          break;
      }
      console.log("this is a message");

      console.log(connStat);
    });

    const unmount = listen<ChatMessage>("ws-message", (event) => {
      const message: ChatMessage = event.payload;
      setMessages((prev) => [...prev, message]);
    });

    return () => {
      unmount.then((fn) => fn());
      unmountConnStat.then((fn) => fn());
    };
  }, []);

  const handleClick = async () => {
    await invoke("send_message", { msg: msg_out, id: "Oscar" });
  };

  const handleInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    setMsg_out(e.target.value);
  };
  return (
    <div className="container-fluid main">
      <div className="row h-100">
        <div className="col-2 menu"> Welcome to Ping!</div>
        <div className="col chat-view">
          <span className="input-field">
            <input onChange={handleInput} placeholder="Type Messsage Here" />
            <button onClick={handleClick}>send</button>
          </span>

          <span>
            {messages.map((msg, index) => (
              <Message key={index} msg={msg} />
            ))}
          </span>
        </div>
      </div>
    </div>
  );
};

export default App;
