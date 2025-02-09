import { SecretNetworkClient, Wallet } from "secretjs";

const wallet = new Wallet("desk pigeon hammer sleep only mistake stool december offer patrol once vacant");

const secretjs = new SecretNetworkClient({
    chainId: "pulsar-3",
    url: "https://api.pulsar3.scrttestnet.com",
    wallet: wallet,
    walletAddress: wallet.address,
  });

let try_flip = async () => {
    const flip_tx = await secretjs.tx.compute.executeContract(
      {
        sender: wallet.address,
        contract_address: "secret194jga9whdutqsjxpvjxg0ykvm09290ql7qdudx",
        msg: {
          flip: {},
        },
        code_hash: "ff6dd6a0e1dc8950ff74d12b07a3dc14c50dae3812f335f1d61e5f08226e662e",
      },
      { gasLimit: 100_000 }
    );
  
    console.log(flip_tx);
  };
  try_flip();