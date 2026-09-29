# Sağlık, renkler ve durum

Her sunucunun tam olarak bir **durumu** vardır. Bu durum, kutucuğun rengini ve
duruma göre sıralarken sırayı belirler.

| Durum | Renk | Anlamı |
| --- | --- | --- |
| `unreachable` | eflatun | skry bağlanamıyor ya da sunucuyu okuyamıyor |
| `critical` | kırmızı | en az bir metrik **kritik** eşiğinde ya da üstünde |
| `warning` | sarı | en az bir metrik **uyarı** eşiğinde ya da üstünde |
| `deviation` | camgöbeği | tüm eşikler yolunda ama bir değer **bu sunucu için olağandışı** ([Baseline'lar](baselines.md)) |
| `ok` | yeşil | her şey sınırlar içinde |
| `pending` | gri | bağlandı ya da bağlanıyor, ilk ölçüm henüz gelmedi |

Birden çok şey aynı anda doğruysa **en kötüsü** kazanır; tablodaki sırayla:
unreachable > critical > warning > deviation > ok.

## Eşikler

Dört metrik sabit eşiklerle karşılaştırılır:

| Metrik | Karşılaştırılan | Uyarı | Kritik |
| --- | --- | --- | --- |
| `cpu` | CPU % (tüm çekirdekler) | 80 | 95 |
| `memory` | kullanılan bellek % | 85 | 95 |
| `disk` | en dolu gerçek dosya sistemi, % | 80 | 90 |
| `load_per_core` | 1 dakikalık yük ÷ çekirdek | 1.5 | 3.0 |

Sınıra **eşit** bir değer sınıra ulaşmış sayılır (`≥`). Sınırları yapılandırma
dosyasında değiştirin:

```toml
[thresholds.disk]
warning = 85.0
critical = 95.0
```

`warning` ve `critical` ikisi de verilmelidir ve `warning`, `critical`'dan
büyük olmamalıdır.

## Durumu *değiştirmeyen* şeyler

- **Güvenlik bulguları** (başarısız girişler, bekleyen güvenlik güncellemeleri,
  beklenmeyen portlar) `⚑` işaretiyle ve güvenlik nabzında gösterilir, uyarı
  gönderebilir; ama kutucuğu sarıya ya da kırmızıya çevirmez. Bir sunucu gayet
  sağlıklıyken yine de yamaya ihtiyaç duyabilir.
- **Çökmüş systemd birimleri** `✖ N unit` işaretiyle gösterilir.
- **`n/a` olan bölümler** (örneğin journal'ı okuma izni yok) bir sunucuyu asla
  sağlıksız yapmaz.

## Ping atabildiğim hâlde sunucum neden "unreachable"?

"Unreachable", yalnızca "ağ yok" değil, *skry veri alamıyor* demektir.
Kutucuk nedenini şu etiketlerden biriyle söyler:

| Etiket | Anlamı |
| --- | --- |
| `unreachable` | ağ sorunu: isim bulunamadı, bağlantı reddedildi, rota yok |
| `timeout` | zamanında cevap yok (paketleri düşüren güvenlik duvarı, aşırı yüklü sunucu) |
| `auth failed` | sunucu sunulan tüm anahtarları reddetti |
| `host key` | bilinmeyen, değişmiş ya da iptal edilmiş host anahtarı |
| `ssh error` | protokol hatası ya da uzak komut kullanılabilir çıktı üretmedi |
| `config` | hedef çözümlenemedi (ör. ProxyJump döngüsü) |

[Bağlantılar, hatalar ve yeniden bağlanma](connections.md) ile
[Sorun giderme](troubleshooting.md) sayfalarına bakın.
