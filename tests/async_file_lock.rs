#![deny(unused_must_use)]
#![feature(assert_matches)]

use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::test;
use {async_file_lock::FileLock, std::assert_matches::assert_matches};

fn file() -> FileLock {
    FileLock::new_std(tempfile::tempfile().unwrap())
}

#[test(flavor = "multi_thread")]
async fn normal1() {
    // let tmp_path = NamedTempFile::new()?.into_temp_path();
    // // let tmp_path = PathBuf::from("/tmp/a");
    // match fork() {
    //     Ok(Fork::Parent(_)) => {
    //         // println!("parent {}", tmp_path.exists());
    //         let mut buf = String::new();
    //         let mut file = FileLock::create(&tmp_path).await?;
    //         // println!("parent {}", tmp_path.exists());
    //         println!("writen {}", file.write(b"a").await?);
    //         std::thread::sleep(std::time::Duration::from_millis(200));
    //         println!("slept");
    //         println!("sought {}", file.seek(SeekFrom::Start(0)).await?);
    //         file.read_to_string(&mut buf).await?;
    //         println!("read");
    //         // // We write at location 0 then it gets overridden.
    //         assert_eq!(buf, "b");
    //     }
    //     Ok(Fork::Child) => {
    //         std::thread::sleep(std::time::Duration::from_millis(100));
    //         println!("child {}", tmp_path.exists());
    //         let mut file = FileLock::open(&tmp_path).await?;
    //         println!("child opened");
    //         file.write(b"b").await?;
    //         println!("done");
    //         std::process::exit(0);
    //     }
    //     Err(_) => panic!("unable to fork"),
    // }
    // Ok(())
    let file = NamedTempFile::new().unwrap();

    let tmp_path: PathBuf = file.path().into();
    let tmp_path2 = tmp_path.clone();

    // signal channels
    let (writen_tx, writen_rx) = tokio::sync::oneshot::channel::<()>();
    let (overwriten_tx, overwriten_rx) = tokio::sync::oneshot::channel::<()>();

    // child thread
    tokio::spawn(async move {
        // open file
        let mut file = FileLock::open(&tmp_path).await.unwrap();
        assert!(tmp_path.exists());

        // write to file
        assert_matches!(file.write(b"a").await, Ok(_));
        assert_matches!(writen_tx.send(()), Ok(()));

        // wait for overwriten
        assert_matches!(overwriten_rx.await, Ok(()));

        // read content
        let mut buf = String::new();
        assert_matches!(file.read_to_string(&mut buf).await, Ok(_));

        // assert it has been overwriten
        assert_eq!(buf, "b");
    });

    // main thread
    {
        // open file
        let mut file = FileLock::open(&tmp_path2).await.unwrap();
        assert!(tmp_path2.exists());

        // wait for writen signal
        assert_matches!(writen_rx.await, Ok(()));

        // overwrite
        assert_matches!(file.write(b"b").await, Ok(_));

        // signal
        assert_matches!(overwriten_tx.send(()), Ok(()));
    };
}

#[test(flavor = "multi_thread")]
async fn append_mode() {
    // let tmp_path = NamedTempFile::new()?.into_temp_path();
    // match fork() {
    //     Ok(Fork::Parent(_)) => {
    //         let mut buf = String::new();
    //         let mut file = FileLock::create(&tmp_path).await?;
    //         file.set_seeking_mode(SeekFrom::End(0));
    //         file.write(b"a").await?;
    //         // println!("parent {}", tmp_path.exists());
    //         std::thread::sleep(std::time::Duration::from_millis(200));
    //         file.write(b"a").await?;
    //         file.seek(SeekFrom::Start(0)).await?;
    //         // Turn off auto seeking mode
    //         file.set_seeking_mode(SeekFrom::Current(0));
    //         file.read_to_string(&mut buf).await?;
    //         // Each file handle has its own position.
    //         assert_eq!(buf, "bba");
    //     }
    //     Ok(Fork::Child) => {
    //         std::thread::sleep(std::time::Duration::from_millis(100));
    //         // println!("{}", tmp_path.exists());
    //         let mut file = FileLock::open(&tmp_path).await?;
    //         file.write(b"bb").await?;
    //         file.flush().await?;
    //         // println!("done");
    //         std::process::exit(0);
    //     }
    //     Err(_) => panic!("unable to fork"),
    // }
    // Ok(())
    let file = NamedTempFile::new().unwrap();

    let tmp_path: PathBuf = file.path().into();
    let tmp_path2 = tmp_path.clone();

    // signal channels
    let (writen_tx, writen_rx) = tokio::sync::oneshot::channel::<()>();
    let (overwriten_tx, overwriten_rx) = tokio::sync::oneshot::channel::<()>();

    // child thread
    tokio::spawn(async move {
        // open file
        let mut file = FileLock::open(&tmp_path).await.unwrap();
        assert!(tmp_path.exists());

        file.set_seeking_mode(SeekFrom::End(0));

        // write to file
        assert_matches!(file.write(b"a").await, Ok(_));
        assert_matches!(writen_tx.send(()), Ok(()));

        // wait for overwriten
        assert_matches!(overwriten_rx.await, Ok(()));

        // write, it should append to the end
        assert_matches!(file.write(b"a").await, Ok(_));

        assert_matches!(file.seek(SeekFrom::Current(0)).await, Ok(_));

        // turn off auto seeking mode
        file.set_seeking_mode(SeekFrom::Current(0));

        // read content
        let mut buf = String::new();
        assert_matches!(file.read_to_string(&mut buf).await, Ok(_));

        // assert it has been overwriten
        assert_eq!(buf, "bba");
    });

    // main thread
    {
        // wait for writen signal
        assert_matches!(writen_rx.await, Ok(()));

        // open file
        let mut file = FileLock::open(&tmp_path2).await.unwrap();
        assert!(tmp_path2.exists());

        // overwrite
        assert_matches!(file.write(b"bb").await, Ok(_));
        assert_matches!(file.flush().await, Ok(_));

        // signal
        assert_matches!(overwriten_tx.send(()), Ok(()));
    };
}

#[test(flavor = "multi_thread")]
async fn lock_shared() {
    // let tmp_path = NamedTempFile::new()?.into_temp_path();
    // match fork() {
    //     Ok(Fork::Parent(_)) => {
    //         std::thread::sleep(std::time::Duration::from_millis(100));
    //         let mut file = FileLock::open(&tmp_path).await?;
    //         let instant = Instant::now();
    //         file.lock_shared().await?;
    //         assert!(instant.elapsed().as_millis() < 100);
    //     }
    //     Ok(Fork::Child) => {
    //         let mut file = FileLock::open(&tmp_path).await?;
    //         file.lock_shared().await?;
    //         std::thread::sleep(std::time::Duration::from_millis(1000));
    //         std::process::exit(0);
    //     }
    //     Err(_) => panic!("unable to fork"),
    // }
    // Ok(())
    let file = NamedTempFile::new().unwrap();

    let tmp_path: PathBuf = file.path().into();
    let tmp_path2 = tmp_path.clone();

    // signal channels
    let (locked_tx, locked_rx) = tokio::sync::oneshot::channel::<()>();
    let (unlocked_tx, unlocked_rx) = tokio::sync::oneshot::channel::<()>();

    // child thread
    tokio::spawn(async move {
        // wait for file locked by main thread
        assert_matches!(locked_rx.await, Ok(()));

        assert!(tmp_path.exists());
        let mut file = FileLock::open(&tmp_path).await.unwrap();

        // attemp to obtain a lock acquired should failed.
        assert_matches!(file.try_lock_shared(), Ok(_));

        // wait for lock released from main thread
        assert_matches!(unlocked_rx.await, Ok(()));

        // attemp to obtain a lock should success.
        assert_matches!(file.try_lock_shared(), Ok(()));
    });

    // main thread
    {
        let mut file = FileLock::open(&tmp_path2).await.unwrap();
        assert!(tmp_path2.exists());

        // obtain the lock
        assert_matches!(file.lock_shared().await, Ok(_));

        // signal child thread
        assert_matches!(locked_tx.send(()), Ok(()));

        // sleep
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        // unlock
        file.unlock().await;

        // signal
        assert_matches!(unlocked_tx.send(()), Ok(()));
    };
}

#[test(flavor = "multi_thread")]
async fn lock_exclusive() {
    let file = NamedTempFile::new().unwrap();

    let tmp_path: PathBuf = file.path().into();
    let tmp_path2 = tmp_path.clone();

    // signal channels
    let (locked_tx, locked_rx) = tokio::sync::oneshot::channel::<()>();
    let (unlocked_tx, unlocked_rx) = tokio::sync::oneshot::channel::<()>();

    // child thread
    tokio::spawn(async move {
        // wait for file locked by main thread
        assert_matches!(locked_rx.await, Ok(()));

        assert!(tmp_path.exists());
        let mut file = FileLock::open(&tmp_path).await.unwrap();

        // attemp to obtain a lock acquired should failed.
        assert_matches!(file.try_lock_exclusive(), Err(_));

        // wait for lock released from main thread
        assert_matches!(unlocked_rx.await, Ok(()));

        // attemp to obtain a lock should success.
        assert_matches!(file.try_lock_exclusive(), Ok(()));
    });

    // main thread
    {
        let mut file = FileLock::open(&tmp_path2).await.unwrap();
        assert!(tmp_path2.exists());

        // obtain the lock
        assert_matches!(file.lock_exclusive().await, Ok(_));

        // signal child thread
        assert_matches!(locked_tx.send(()), Ok(()));

        // sleep
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        // unlock
        file.unlock().await;

        // signal
        assert_matches!(unlocked_tx.send(()), Ok(()));
    };
}

#[test(flavor = "multi_thread")]
async fn lock_exclusive_shared() {
    // let tmp_path = NamedTempFile::new()?.into_temp_path();
    // match fork() {
    //     Ok(Fork::Parent(_)) => {
    //         std::thread::sleep(std::time::Duration::from_millis(100));
    //         let mut file = FileLock::open(&tmp_path).await?;
    //         let instant = Instant::now();
    //         file.lock_exclusive().await?;
    //         assert!(instant.elapsed().as_millis() > 800);
    //     }
    //     Ok(Fork::Child) => {
    //         let mut file = FileLock::open(&tmp_path).await?;
    //         file.lock_shared().await?;
    //         std::thread::sleep(std::time::Duration::from_millis(1000));
    //         std::process::exit(0);
    //     }
    //     Err(_) => panic!("unable to fork"),
    // }
    // Ok(())

    let file = NamedTempFile::new().unwrap();

    let tmp_path: PathBuf = file.path().into();
    let tmp_path2 = tmp_path.clone();

    // signal channels
    let (locked_tx, locked_rx) = tokio::sync::oneshot::channel::<()>();
    let (unlocked_tx, unlocked_rx) = tokio::sync::oneshot::channel::<()>();

    // child thread
    tokio::spawn(async move {
        // wait for file locked by main thread
        assert_matches!(locked_rx.await, Ok(()));

        assert!(tmp_path.exists());
        let mut file = FileLock::open(&tmp_path).await.unwrap();

        // attemp to obtain a lock acquired should failed.
        assert_matches!(file.try_lock_exclusive(), Err(_));

        // wait for lock released from main thread
        assert_matches!(unlocked_rx.await, Ok(()));

        // attemp to obtain a lock should success.
        assert_matches!(file.try_lock_exclusive(), Ok(()));
    });

    // main thread
    {
        let mut file = FileLock::open(&tmp_path2).await.unwrap();
        assert!(tmp_path2.exists());

        // obtain the lock
        assert_matches!(file.lock_shared().await, Ok(_));

        // signal child thread
        assert_matches!(locked_tx.send(()), Ok(()));

        // sleep
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        // unlock
        file.unlock().await;

        // signal
        assert_matches!(unlocked_tx.send(()), Ok(()));
    };
}

#[test(flavor = "multi_thread")]
async fn drop_lock_exclusive() {
    let file = NamedTempFile::new().unwrap();

    let tmp_path: PathBuf = file.path().into();
    let tmp_path2 = tmp_path.clone();

    // signal channels
    let (locked_tx, locked_rx) = tokio::sync::oneshot::channel::<()>();
    let (unlocked_tx, unlocked_rx) = tokio::sync::oneshot::channel::<()>();

    // child thread
    tokio::spawn(async move {
        // wait for file locked by main thread
        assert_matches!(locked_rx.await, Ok(()));

        assert!(tmp_path.exists());
        let mut file = FileLock::open(&tmp_path).await.unwrap();

        // attemp to obtain a lock acquired should failed.
        assert_matches!(file.try_lock_exclusive(), Err(_));

        // wait for lock released from main thread
        assert_matches!(unlocked_rx.await, Ok(()));

        // attemp to obtain a lock should success.
        assert_matches!(file.try_lock_exclusive(), Ok(()));
    });

    // main thread
    {
        let mut file = FileLock::open(&tmp_path2).await.unwrap();
        assert!(tmp_path2.exists());

        // obtain the lock
        assert_matches!(file.lock_exclusive().await, Ok(_));

        // signal child thread
        assert_matches!(locked_tx.send(()), Ok(()));

        // sleep
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        // drop lock
        drop(file);

        // signal
        assert_matches!(unlocked_tx.send(()), Ok(()));
    };
}

#[test(flavor = "multi_thread")]
#[should_panic]
async fn exclusive_locking_locked_file_panics() {
    let mut file = file();
    file.lock_exclusive().await.unwrap();
    file.lock_exclusive().await.unwrap();
}

#[test(flavor = "multi_thread")]
#[should_panic]
async fn shared_locking_locked_file_panics() {
    let mut file = file();
    file.lock_shared().await.unwrap();
    file.lock_shared().await.unwrap();
}

#[test(flavor = "multi_thread")]
#[should_panic]
async fn shared_locking_exclusive_file_panics() {
    let mut file = file();
    file.lock_exclusive().await.unwrap();
    file.lock_shared().await.unwrap();
}

#[test(flavor = "multi_thread")]
#[should_panic]
async fn exclusive_locking_shared_file_panics() {
    let mut file = file();
    file.lock_shared().await.unwrap();
    file.lock_exclusive().await.unwrap();
}

#[test(flavor = "multi_thread")]
#[should_panic]
async fn unlocking_file_panics() {
    let mut file = file();
    file.unlock().await;
}

#[test(flavor = "multi_thread")]
#[should_panic]
async fn unlocking_unlocked_file_panics() {
    let mut file = file();
    file.lock_exclusive().await.unwrap();
    file.unlock().await;
    file.unlock().await;
}

#[test(flavor = "multi_thread")]
async fn file_stays_locked() {
    let mut file = file();
    file.lock_exclusive().await.unwrap();
    file.write(b"a").await.unwrap();
    file.unlock().await;
}

#[test(flavor = "multi_thread")]
#[should_panic]
async fn file_auto_unlocks() {
    let mut file = file();
    file.write(b"a").await.unwrap();
    file.unlock().await;
}
