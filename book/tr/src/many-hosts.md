# Çok sayıda sunucu ve gruplar

## Komut satırında sunucuları listelemek

skry'ye istediğiniz kadar sunucu verebilir, biçimleri karıştırabilirsiniz:

```sh
skry web1 web2 db1 deploy@203.0.113.50 root@[2001:db8::7]:2222
```

Kabul edilen biçimler:

| Biçim | Örnek |
| --- | --- |
| `~/.ssh/config` içindeki bir isim | `web1` |
| Düz adres (yerel kullanıcı adınız, port 22) | `203.0.113.10` |
| kullanıcı ve adres | `deploy@203.0.113.10` |
| kullanıcı, adres ve port | `deploy@203.0.113.10:2222` |
| Portlu IPv6 | `root@[2001:db8::7]:2222` |
| Portsuz IPv6 | `root@2001:db8::7` |
| URI biçimi | `ssh://deploy@203.0.113.10:2222` |
| Yapılandırma dosyasındaki bir grup | `@production` |

Açıkça yazdığınız (kullanıcı, port) her şey `~/.ssh/config`'e üstün gelir.
Aynı sunucu iki kez verilirse yalnızca bir kez izlenir.

## Gruplar

Her seferinde yirmi isim yazmak eğlenceli değildir. Onları skry yapılandırma
dosyasında bir gruba koyun (dosyanın nerede olduğu için
[Yapılandırma](configuration.md) sayfasına bakın; `skry config path` yolu
yazdırır):

```toml
[groups.web]
hosts = ["web1", "web2", "web3"]

[groups.databases]
hosts = ["db1", "db2", "deploy@10.0.5.20:2222"]

[groups.production]
hosts = ["@web", "@databases", "cache1"]
```

Artık:

```sh
skry @web            # üç sunucu
skry @production     # yedi sunucu: web1-3, db1-2, 10.0.5.20, cache1
skry @web db-test    # gruplar ve tek sunucular karıştırılabilir
```

Gruplar başka grupları (`@` ile) içerebilir. Doğrudan ya da dolaylı olarak
kendini içeren bir grup sonsuz döngüye girmek yerine hata olarak bildirilir.

## Grupların başka işleri

Bir grup ayrıca bir **politika** taşıyabilir:

```toml
[groups.web]
hosts = ["web1", "web2"]
allowed_ports = [22, 80, 443]          # başka bir port dinlerse işaretlenir
tls = ["www.example.com:443"]          # sertifika bitiş tarihi kontrol edilir
```

- `allowed_ports` — [güvenlik nabzında](security-pulse.md), bu gruptaki bir
  sunucu (loopback olmayan bir adreste) başka bir port dinliyorsa işaretlenir.
  Bir sunucu birden çok gruptaysa, hepsinin izinli portları birleştirilir.
- `tls` — grup izlendiği sürece bu HTTPS uç noktaları kontrol edilir.

## skry kaç sunucuyu kaldırır?

skry varsayılan olarak **aynı anda** en fazla 32 sunucuyla temas kurar
(`concurrency` ayarı). Bu, sunucu sayısına değil eşzamanlı işe konan bir
sınırdır: 200 sunucuyla skry onları paralel gruplar hâlinde işler. Her sunucu
ölçümler arasında yine kendi SSH bağlantısını açık tutar.

Yavaş ya da bozuk bir sunucu diğerlerini asla geciktirmez: her sunucunun kendi
bağımsız işçisi vardır. Boyutlandırma için [Performans ve yük](performance.md)
sayfasına bakın.

## Terminal arayüzünde büyük bir filoda gezinmek

| Tuş | İşlem |
| --- | --- |
| `/` ve ardından metin | Yalnızca adında ya da grubunda bu metin geçen sunucuları göster |
| `f` | Sırayla: hepsi → yalnızca sorunlular → yalnızca erişilemeyenler |
| `o` | Sıralama: durum (en kötüsü önce) → isim → CPU → bellek → disk |
| `Esc` | Filtreleri temizle |

Üst satır her zaman toplamları gösterir, örneğin
`12 hosts ● 9 ok ● 1 warning ● 2 unreachable`.
