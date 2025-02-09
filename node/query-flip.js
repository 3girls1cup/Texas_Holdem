import { SecretNetworkClient, Wallet } from "secretjs";

const wallet = new Wallet("desk pigeon hammer sleep only mistake stool december offer patrol once vacant");

const secretjs = new SecretNetworkClient({
    chainId: "pulsar-3",
    url: "https://api.pulsar3.scrttestnet.com",
    wallet: wallet,
    walletAddress: wallet.address,
  });

  let query_flip = async () => {
    let flip_tx = await secretjs.query.compute.queryContract({
      contract_address: "secret194jga9whdutqsjxpvjxg0ykvm09290ql7qdudx",
      code_hash: "ff6dd6a0e1dc8950ff74d12b07a3dc14c50dae3812f335f1d61e5f08226e662e",
      query: {
        get_flip: {},
      },
    });
    console.log(flip_tx);
  };
  
  query_flip();