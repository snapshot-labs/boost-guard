export default class Strategy {
  public boostId;
  public recipient;
  public tag;
  public params;

  constructor(boostId, recipient, tag, params) {
    this.boostId = boostId;
    this.recipient = recipient;
    this.tag = tag;
    this.params = params;
  }
}
