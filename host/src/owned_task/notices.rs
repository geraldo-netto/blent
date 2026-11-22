//! Shared task/notice readiness adapter. The original Task is the sole join
//! owner; no separate terminal flag can outlive or precede that join.
use super::*;
use tokio::sync::mpsc;

impl Task {
    /// The original task owns the bounded notice producers. Drain queued notices
    /// first, then observe its join. Cancellation retains both original owners;
    /// an empty/closed channel alone never establishes task/native retirement.
    pub(crate) fn poll_notice<T>(
        &mut self,
        notices: &mut mpsc::Receiver<T>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<T>> {
        if self.is_joined() {
            return Poll::Ready(Err(anyhow::anyhow!("notice task already joined")));
        }
        if let Poll::Ready(Some(notice)) = notices.poll_recv(cx) {
            return Poll::Ready(Ok(notice));
        }
        std::task::ready!(Pin::new(self).poll(cx))??;
        Poll::Ready(Err(anyhow::anyhow!("notice task exited")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn notice<T>(task: &mut Task, input: &mut mpsc::Receiver<T>) -> Result<T> {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            std::future::poll_fn(|cx| task.poll_notice(input, cx)),
        )
        .await
        .unwrap()
    }
    #[tokio::test]
    async fn t1778_notices_drain_before_one_exact_successful_join() {
        let (output, mut input) = mpsc::channel(2);
        let mut task = Task::spawn(async move {
            output.send(7).await.unwrap();
            output.send(8).await.unwrap();
            Ok(())
        })
        .unwrap();
        let completion = task.completion();
        assert_eq!(notice(&mut task, &mut input).await.unwrap(), 7);
        assert_eq!(notice(&mut task, &mut input).await.unwrap(), 8);
        assert!(!task.is_joined());
        assert!(notice(&mut task, &mut input).await.is_err());
        assert!(task.is_joined());
        assert!(completion.is_complete());
        assert!(notice(&mut task, &mut input).await.is_err());
    }
    #[tokio::test]
    async fn t1778_open_notice_channel_does_not_hide_failed_or_cancelled_task_join() {
        for cancel in [false, true] {
            let (_output, mut input) = mpsc::channel::<u8>(1);
            let mut task =
                Task::spawn(async { anyhow::bail!("T1778 injected task failure") }).unwrap();
            let completion = task.completion();
            if cancel {
                task.abort();
            }
            assert!(notice(&mut task, &mut input).await.is_err());
            assert!(task.is_joined());
            assert!(completion.is_complete());
            assert!(notice(&mut task, &mut input).await.is_err());
        }
    }
}
