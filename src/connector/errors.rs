pub enum ConnectorError{
    FailWebsocketConnection,
    WebsocketDisconnected,
    ParsingTradeError,
    ParsingLevel2Error,
}