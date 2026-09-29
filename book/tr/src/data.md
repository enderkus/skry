# skry'nin bilgisayarınızda tuttuğu dosyalar

skry sunucularınıza hiçbir şey yazmaz. Kendi bilgisayarınızda şu dosyaları
kullanabilir:

| Dosya | Amaç | Varsayılan konum |
| --- | --- | --- |
| Yapılandırma | gruplarınız, eşikleriniz, webhook'larınız | bkz. [Yapılandırma](configuration.md#dosya-nerede) |
| Geçmiş veritabanı | metrik geçmişi ve baseline'lar | Linux `~/.local/share/skry/history.db`, macOS `~/Library/Application Support/skry/history.db`, Windows `%APPDATA%\skry\data\history.db` |
| `known_hosts` | `--accept-new` ile kabul ettiğiniz host anahtarları | `~/.ssh/known_hosts` (ya da `UserKnownHostsFile`'ınız) |
| Anlık görüntüler | Markdown ve JSON raporları | geçerli dizin, `snapshot.dir` ya da `--out` |
| Log dosyası | yalnızca `--log-file` verirseniz | nereyi söylerseniz |

skry çalışırken geçmiş veritabanının yanında `history.db-wal` ve
`history.db-shm` dosyaları olabilir; bunlar normal SQLite çalışma dosyalarıdır.

## Her şeyi kaldırmak

```sh
# Linux
rm -rf ~/.local/share/skry ~/.config/skry
# macOS
rm -rf ~/Library/Application\ Support/skry ~/.config/skry
```

Windows'ta `%APPDATA%\skry` klasörünü silin. Programın kendisini
[Kurulum](installation.md#kaldırma) sayfasında anlatıldığı gibi kaldırın.
`known_hosts` içindeki girdiler standart OpenSSH girdileridir; saklayın
(`ssh`'iniz de kullanır) ya da `ssh-keygen -R <sunucu>` ile silin.
