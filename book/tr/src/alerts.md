# Uyarılar ve webhook'lar

skry bir şey ters gittiğinde — ve düzeldiğinde — Slack'e, Discord'a ya da
herhangi bir HTTP uç noktasına haber verebilir. Uyarıları, çalıştıkları sürece
terminal arayüzü ve `skry serve` gönderir. (Tek seferlik komutlar uyarı
göndermez.)

## Uyarıyı ne tetikler

| Kategori | Koşul | Seviye |
| --- | --- | --- |
| `threshold` | bir metrik uyarı ya da kritik eşiğine ulaşır | warning / critical |
| `baseline` | bir değer sunucunun baseline'ından sapar | warning |
| `unreachable` | bir sunucu `alerts.unreachable_after` saniyedir (varsayılan 60) hata veriyor | critical |
| `security` | bir güvenlik bulgusu (başarısız girişler, güvenlik güncellemeleri, beklenmeyen portlar) ya da TLS sertifikası sorunu | warning / critical |

Her uyarının sabit bir adı vardır, örneğin `threshold:cpu`,
`baseline:memory`, `unreachable`, `security:failed_logins`,
`tls_certificate`.

## Bir uyarının hayatı

```text
 koşul başlar ──▶ FIRING (tetiklendi) bildirimi
 hâlâ doğru, bekleme süresi dolmadı ──▶ (sessizlik)
 kötüleşir (warning → critical) ──▶ ESCALATED (yükseldi) bildirimi, hemen
 bekleme süresinden sonra hâlâ doğru ──▶ REMINDER (hatırlatma) bildirimi
 koşul kalkar ──▶ RESOLVED (çözüldü) bildirimi
 bekleme süresi içinde geri gelir ──▶ (bastırılır: gidip gelme koruması)
```

- **Tekilleştirme**: bir koşul doğru kaldığı sürece ölçüm başına değil, tek
  bir bildirim alırsınız.
- **Bekleme süresi** (`alerts.cooldown`, varsayılan 900 sn = 15 dk): aynı
  sunucu ve uyarı için iki bildirim arasındaki en kısa süre. Gidip gelmeye karşı
  da korur — %89 → %91 → %89 → %91 arasında oynayan bir disk bildirim fırtınası
  yaratmaz.
- **Yükselme** her zaman hemen iletilir.
- **Çözüldü** mesajları yalnızca tetiklenme bildirimi gerçekten gönderildiyse
  (ve varsayılan olduğu gibi `notify_resolved = true` ise) gönderilir.
- Bir sunucu erişilemezken diğer uyarıları olduğu gibi tutulur; böylece bir
  sunucu çöktüğünde yanıltıcı "çözüldü" mesajlarından oluşan bir sel almazsınız.

## Slack kurulumu

1. Slack'te bir kanal için *Incoming Webhook* oluşturun
   (Apps → Incoming Webhooks) ve adresini kopyalayın.
2. Yapılandırma dosyasına ekleyin:

   ```toml
   [[webhooks]]
   name = "ops-slack"
   kind = "slack"
   url = "https://hooks.slack.com/services/<webhook-yolunuz>"
   ```

Bir mesaj şöyle görünür:

```text
🔴 [CRITICAL] db-1: threshold:cpu — cpu 97.2% ≥ 95.0% (critical)
✅ [CRITICAL] db-1: threshold:cpu resolved — cpu 97.2% ≥ 95.0% (critical)
```

## Discord kurulumu

Kanal ayarlarında *Integrations → Webhooks → New Webhook*, adresi kopyalayın:

```toml
[[webhooks]]
kind = "discord"
url = "https://discord.com/api/webhooks/<id>/<token>"
```

2000 karakterden uzun mesajlar kısaltılır (Discord sınırı).

## Genel JSON webhook'u

Kendi araçlarınız, PagerDuty/Opsgenie köprüleri, n8n vb. için:

```toml
[[webhooks]]
name = "pager"
kind = "generic"
url = "https://alerts.example.com/skry"
```

skry, `Content-Type: application/json` ile bir HTTP `POST` gönderir:

```json
{
  "source": "skry",
  "kind": "firing",
  "host": "db-1",
  "alert": "threshold:cpu",
  "category": "threshold",
  "level": "critical",
  "message": "cpu 97.2% ≥ 95.0% (critical)",
  "timestamp": "2026-09-29T12:00:00Z"
}
```

`kind` şunlardan biridir: `firing`, `reminder`, `escalated`, `resolved`.
`level`, `warning` ya da `critical`'dır. TLS uyarılarında `host`,
`tls:<uç nokta>` olur.

## Neyin nereye gideceğini seçmek

`events`, bir webhook'u bazı kategorilerle sınırlar (verilmezse dördü de):

```toml
[[webhooks]]
name = "guvenlik-ekibi"
kind = "slack"
url = "https://hooks.slack.com/services/<yol-1>"
events = ["security"]

[[webhooks]]
name = "nobetci"
kind = "generic"
url = "https://alerts.example.com/skry"
events = ["threshold", "unreachable"]
```

## Ayarlar

```toml
[alerts]
cooldown = 900           # aynı uyarı için bildirimler arası saniye
notify_resolved = true   # "çözüldü" mesajları gönder
unreachable_after = 60   # "unreachable" tetiklenmeden önceki hata süresi (sn)
```

## Bilmekte fayda var

- İletim hataları (ağ hatası, HTTP hatası) loga (`--log-file`) yazılır ve
  yeniden denenmez. Webhook adresleri asla tam hâliyle yazdırılmaz; loglarda
  yalnızca `https://hooks.slack.com/<redacted>` görünür.
- Webhook adresleri `http://` ya da `https://` ile başlamalıdır.
- Her isteğin 10 saniyelik zaman aşımı vardır.
- Uyarılar için skry'nin çalışıyor olması gerekir. Kesintisiz uyarı için
  `skry serve`'ü bir servis olarak çalıştırın
  ([Betikler ve otomasyon](automation.md)).
