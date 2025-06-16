interface ChatMessage {
  user_id: string;
  timestamp: string;
  content: string;
}

interface Props {
  msg: ChatMessage;
}

const Message = ({ msg }: Props) => {
  return (
    <div>
      <p>
        {msg.user_id}: {msg.content}
      </p>
    </div>
  );
};

export default Message;
