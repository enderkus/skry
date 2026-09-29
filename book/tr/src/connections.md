# Bağlantılar, hatalar ve yeniden bağlanma

## Sunucu başına tek bağlantı

skry her sunucuya **tek** bir SSH bağlantısı açar ve çalıştığı sürece açık
tutar. Her ölçüm bu bağlantının içinde yeni ve hafif bir *kanal* kullanır. Bu,
iki saniyede bir giriş yapmaktan çok daha ucuzdur ve sunucularınızın auth
loglarını sakin tutar: ölçüm başına değil, skry oturumu başına tek giriş.

Ölü bağlantıları (yeniden başlatılmış bir sunucu, kopan bir VPN) fark etmek
için skry her 15 saniyede bir SSH keepalive gönderir. Üç keepalive
cevapsız kalırsa ya da bir ölçüm başarısız olursa bağlantı düşürülür ve yenisi
kurulur.

## Bir şey ters gittiğinde ne olur

```text
            ┌───────────── başarılı ────────────┐
            ▼                                   │
  [bağlan] ──hata──▶ 1 sn bekle ──▶ [bağlan] ──hata──▶ 2 sn bekle ──▶ … 4, 8, 16, 32, 60, 60 sn …
                                                        (üstel geri çekilme, en fazla 60 sn)
```

- Başarısız bir bağlantı denemesinden sonra skry bekler ve tekrar dener: 1 sn,
  2 sn, 4 sn, 8 sn, … denemeler arası en fazla **60 saniye**. Bağlantı
  başarılı olur olmaz bekleme süresi 1 sn'ye döner.
- Kendiliğinden düzelemeyecek sorunlar — bir **host anahtarı** sorunu ya da
  bir **yapılandırma** hatası — yalnızca 60 saniyede bir yeniden denenir;
  böylece skry sizin bakmanız gereken bir sunucuyu boşuna yormaz.
- Var olan bir bağlantıda bir **ölçüm** başarısız olursa (zaman aşımı, kopan
  bağlantı), bağlantı kapatılır ve aynı geri çekilmeyle yeniden kurulur.
- Her sunucu bağımsız olarak yeniden dener. Bozuk bir sunucu diğerlerini asla
  yavaşlatmaz.

Sunucu hata verdiği sürece kutucuğu nedenle birlikte eflatun kalır.
Düzeldiğinde bir sonraki ölçümde normale döner. Uyarıları kurduysanız,
"unreachable" uyarısı ancak sunucu `alerts.unreachable_after` saniye
(varsayılan 60) boyunca hata verdikten sonra gönderilir; kısa bir kesinti
kimseyi uykusundan uyandırmaz.

## Hata mesajlarının açıklaması

| Gördüğünüz | Anlamı | Ne yapmalı |
| --- | --- | --- |
| `cannot resolve web1: …` | İsim `~/.ssh/config`'te yok ve DNS de bilmiyor | Yazımı kontrol edin, bir `Host` girdisi ekleyin ya da IP kullanın |
| `cannot connect to …: Connection refused` | O portta dinleyen yok | sshd çalışıyor mu? Port doğru mu (`~/.ssh/config`'teki `Port`)? |
| `… No route to host` / `Network is unreachable` | Yönlendirme ya da VPN sorunu | Ağınızı ya da VPN'inizi kontrol edin |
| `timed out while connecting` | `connect_timeout` (10 sn) içinde cevap yok | Paket düşüren güvenlik duvarı, yanlış adres, kapalı sunucu |
| `timed out while running the collection command` | Giriş yapıldı ama betik `command_timeout`'tan (30 sn) uzun sürdü | Sunucu aşırı yüklü; `command_timeout`'u artırın |
| `unknown host key for …` | Sunucunun anahtarı `known_hosts`'ta yok | Doğrulayıp `--accept-new` kullanın, bkz. [Host anahtarları](host-keys.md) |
| `HOST KEY MISMATCH for …` | Anahtar kayıtlı olandan farklı | **Durun ve araştırın**, bkz. [Host anahtarları](host-keys.md) |
| `host key … is marked @revoked` | `known_hosts` bu anahtarı açıkça iptal ediyor | Sunucuyu yöneten kişiye sorun |
| `authentication failed for user@host (tried N keys)` | Sunucu tüm anahtarları reddetti | `User`, `IdentityFile`, `authorized_keys`'i kontrol edin; bkz. [Kimlik doğrulama](authentication.md) |
| `… (no usable keys: start ssh-agent or set IdentityFile)` | skry'nin sunacak hiç anahtarı yoktu | Ajanınızı başlatın (`ssh-add`) ya da `IdentityFile` ayarlayın |
| `via jump host bastion: …` | Sorun hedefte değil, atlama sunucusunda oldu | Önce bastion erişimini düzeltin |
| `remote command failed: …` / `produced no usable output` | Giriş yapıldı ama kabuk betiği çalıştıramadı | Kısıtlı kabuklar (`rbash`, zorunlu komutlar, `nologin`) izlenemez |
| `ProxyJump chain for … is too long or circular` | Atlama sunucuları birbirini döngüyle gösteriyor ya da 8'den fazla atlama var | `~/.ssh/config`'i düzeltin |

## Sunucu tarafında nazik davranış

- Betikteki tek bir komut takılırsa (örneğin takılmış bir NFS bağlamasında
  `df`), 10 saniyelik `timeout` yalnızca o komutu sonlandırır ve bölümü `n/a`
  olur; ölçümün geri kalanı yine gelir.
- Ölçümün tamamı yarıda kesilirse skry, kesilmeden önce gelen bölümleri yine
  kullanır.
- skry'nin kendisi öldürülürse sunucu yalnızca kapanmış bir SSH bağlantısı
  görür. Arkada çalışan hiçbir şey kalmaz.
