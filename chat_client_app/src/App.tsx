import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

listen<string>("ws-message", (event) => {
  const message = event.payload;
  console.log(message);
});

const handleClick = async () => {
  await invoke("send_message", { msg: "name" });
};

const App = () => {
  return (
    <div>
      <span></span>
      <input></input>
      <button onClick={handleClick}>Send</button>
    </div>
  );
};

export default App;
