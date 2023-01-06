use crate::*;
use tokio::sync::watch;

type AckResult = io::Result<Option<watch::Receiver<bool>>>;

impl<S: UdpSender> RudpSender<S> {
    pub async fn send(&self, pkt: Pkt<&[u8]>) -> AckResult {
        self.share.send(PktType::Orig, pkt).await // TODO: splits
    }
}

impl<S: UdpSender> RudpShare<S> {
    pub async fn send(&self, tp: PktType, pkt: Pkt<&[u8]>) -> AckResult {
        let mut buf = Vec::with_capacity(4 + 2 + 1 + 1 + 2 + 1 + pkt.data.len());
        buf.write_u32::<BigEndian>(PROTO_ID)?;
        buf.write_u16::<BigEndian>(*self.remote_id.read().await)?;
        buf.write_u8(pkt.chan as u8)?;

        let mut chan = self.chans[pkt.chan as usize].lock().await;
        let seqnum = chan.seqnum;

        if !pkt.unrel {
            buf.write_u8(PktType::Rel as u8)?;
            buf.write_u16::<BigEndian>(seqnum)?;
        }

        buf.write_u8(tp as u8)?;
        buf.write(pkt.data)?;

        self.send_raw(&buf).await?;

        if pkt.unrel {
            Ok(None)
        } else {
            // TODO: reliable window
            let (tx, rx) = watch::channel(false);
            chan.acks.insert(
                seqnum,
                Ack {
                    tx,
                    rx: rx.clone(),
                    data: buf,
                },
            );
            chan.seqnum += 1;

            Ok(Some(rx))
        }
    }

    pub async fn send_raw(&self, data: &[u8]) -> io::Result<()> {
        self.udp_tx.send(data).await
        // TODO: reset ping timeout
    }
}