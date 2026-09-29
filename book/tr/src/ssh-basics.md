# Beş dakikada SSH temelleri

skry, SSH'yi tam olarak `ssh` komutu gibi kullanır. `ssh sunucum` yazıp parola
girmeden bir kabuğa düşebiliyorsanız, skry ek bir ayar gerektirmeden çalışır.
Bu sayfa işin içindeki parçaları sade bir dille anlatır; böylece ileride
göreceğiniz hata mesajları anlam kazanır.

## Tek paragrafta SSH

SSH, bilgisayarınız (**istemci**) ile bir sunucu arasındaki şifreli bir
bağlantıdır. Bağlanırken iki soru cevaplanır:

1. **"Bu gerçekten benim sunucum mu?"** — Sunucu kimliğini bir **host
   anahtarı** ile kanıtlar. Bilgisayarınız host anahtarlarını `known_hosts`
   adlı bir dosyada hatırlar.
2. **"Bu gerçekten sen misin?"** — Siz kimliğinizi kanıtlarsınız; genellikle
   bir **anahtar çifti** ile (bilgisayarınızda kalan gizli bir anahtar ve
   sunucuya konan açık bir anahtar).

skry'nin her iki sorunun da bir insanın bir şey yazmasına gerek kalmadan,
otomatik cevaplanmasına ihtiyacı vardır. Bu yüzden parolalara değil,
anahtarlara dayanır.

## Anahtar çiftleri

Bir anahtar çifti iki dosyadır, örneğin:

| Dosya | Nerede durur | Gizli mi? |
| --- | --- | --- |
| `~/.ssh/id_ed25519` | Bilgisayarınızda | **Evet.** Asla paylaşmayın. |
| `~/.ssh/id_ed25519.pub` | İçeriği her sunucuda `~/.ssh/authorized_keys` dosyasına eklenir | Hayır |

Henüz yok mu? Oluşturun:

```sh
ssh-keygen -t ed25519
```

Varsayılan konumu kabul etmek için Enter'a basın. İsterseniz bir **parola**
(passphrase; gizli anahtar dosyasını koruyan bir şifre) belirleyebilirsiniz.
skry parola korumalı anahtarları destekler: başlarken parolayı bir kez sorar.

Açık anahtarı bir sunucuya kopyalayın:

```sh
ssh-copy-id deploy@203.0.113.10
```

Bundan sonra `ssh deploy@203.0.113.10` hesap parolası sormadan giriş yapmalı.

## SSH ajanı

Parolayı her seferinde yazmak yorucudur. **SSH ajanı**, bilgisayarınızda
kilidi açılmış anahtarlarınızı bellekte tutan ve giriş isteklerini sizin
yerinize imzalayan küçük bir programdır.

- macOS: ajan kendiliğinden çalışır; `ssh-add --apple-use-keychain ~/.ssh/id_ed25519`
  parolayı anahtar zincirine kaydeder.
- Linux masaüstleri genellikle ajanı kendiliğinden başlatır. Değilse:
  `eval "$(ssh-agent)" && ssh-add`.
- Windows: "OpenSSH Authentication Agent" servisini etkinleştirin, sonra
  `ssh-add` çalıştırın.

Çalışan bir ajan varsa skry onunla konuşur (Linux/macOS'ta `SSH_AUTH_SOCK`
değişkenine, Windows'ta OpenSSH ajan kanalına ya da Pageant'a bakar). Ajan
anahtarınızı tutuyorsa skry'nin parolanıza hiç ihtiyacı olmaz.

## known_hosts

Bir sunucuya ilk kez bağlandığınızda `ssh` şuna benzer bir şey gösterir:

```text
The authenticity of host '203.0.113.10' can't be established.
ED25519 key fingerprint is SHA256:Zm9vYmFy...
Are you sure you want to continue connecting (yes/no)?
```

"yes" demek, sunucunun host anahtarını `~/.ssh/known_hosts` dosyasına yazar.
Bundan sonra sunucu bir gün farklı bir anahtar sunarsa `ssh` yüksek sesle
reddeder — biri sunucunuzun kılığına giriyor olabilir.

skry aynı `known_hosts` dosyasını okur. `ssh` ile daha önce bağlandığınız
sunuculara güvenilir. Yeni sunuculara ise `--accept-new` eklemediğiniz sürece
**bağlanmayı reddeder**; bu seçenek anahtarı ilk seferde kaydeder ("yes"
demek gibi). **Değişmiş** bir anahtar her zaman reddedilir. Ayrıntılar
[Host anahtarları ve known_hosts](host-keys.md) sayfasında.

## ~/.ssh/config: adres yerine isim

`deploy@203.0.113.10 -p 2222` ezberlemek yerine sunuculara `~/.ssh/config`
içinde isim verebilirsiniz:

```text
Host web1
    HostName 203.0.113.10
    User deploy
    Port 2222
    IdentityFile ~/.ssh/id_ed25519

Host db-*
    User postgres
    ProxyJump bastion
```

Artık `ssh web1` çalışır — `skry web1` de. skry, bağlanmak için önemli olan
ayarları anlar:

| Ayar | Ne yapar |
| --- | --- |
| `Host` | Bu bloğun geçerli olduğu isim(ler) ya da desen(ler) (`*` ve `?` joker, hariç tutmak için `!`) |
| `HostName` | Bağlanılacak gerçek adres |
| `User` | Giriş yapılacak hesap |
| `Port` | SSH portu (varsayılan 22) |
| `IdentityFile` | Kullanılacak gizli anahtar(lar) |
| `IdentitiesOnly` | `yes` = yalnızca `IdentityFile` içindeki anahtarları sun |
| `ProxyJump` | Önce başka bir sunucudan geç (bastion) |
| `Include` | Başka yapılandırma dosyalarını da oku |
| `UserKnownHostsFile` | Başka bir `known_hosts` dosyası kullan |
| `HostKeyAlias` | Sunucuyu `known_hosts` içinde başka bir isimle ara |
| `ConnectTimeout` | Bağlantı için beklenecek saniye |

Dosyadaki diğer her şey (örneğin `ForwardAgent` ya da `LocalForward`) skry
tarafından sessizce yok sayılır; hiçbir şeyi bozmaz.

## Beş saniyelik test

skry'yi kullanmadan önce her sunucuyu bir kez düz SSH ile kontrol edin:

```sh
ssh web1 true && echo calisiyor
```

Parola sormadan `calisiyor` yazıyorsa, skry de çalışır. Parola soruyorsa önce
bir anahtar kurun (yukarıya bakın). Host anahtarından şikâyet ediyorsa
[Host anahtarları](host-keys.md) sayfasına bakın.
