export interface Message {
  id: number;
  role: "user" | "assistant";
  content: string;
  status?: "streaming" | "complete" | "error";
}

export interface StreamChunkEvent {
  text: string;
}

export interface StreamDoneEvent {
  message_id: string;
  session_id?: string;
}

export interface StreamErrorEvent {
  code: string;
  message: string;
}
