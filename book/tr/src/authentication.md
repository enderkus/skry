# Kimlik doğrulama: ajan, anahtarlar ve parolalar

skry **yalnızca SSH anahtarlarıyla** giriş yapar. Hesap parolası asla
istemez, kullanmaz, saklamaz. (Klavye etkileşimli giriş, tek kullanımlık
şifreler, GSSAPI/Kerberos ve SSH sertifikaları da desteklenmez.)

## Anahtarlar nereden gelir?

1. Çalışıyorsa **SSH ajanınızdan**.
2. **Anahtar dosyalarından**: o sunucu için `~/.ssh/config`'teki
   `IdentityFile` girdileri. Bir sunucunun `IdentityFile`'ı yoksa skry var olan
   varsayılan dosyaları kullanır: `~/.ssh/id_ed25519`, `~/.ssh/id_ecdsa`,
   `~/.ssh/id_rsa` (OpenSSH ile aynı varsayılanlar).

## Anahtarların denenme sırası

```text
1. ajandaki anahtarlardan, sunucunun IdentityFile anahtarlarıyla eşleşenler
2. ajandaki diğer tüm anahtarlar       (IdentitiesOnly yes ise atlanır)
3. ajan üzerinden zaten sunulmamış anahtar dosyaları
```

Sunucunun kabul ettiği ilk anahtar kazanır.

> [!WARNING]
> Sunucular bağlantı başına sınırlı sayıda denemeye izin verir
> (`MaxAuthTries`, varsayılan **6**). Ajanınızda çok sayıda anahtar varsa
> sunucu doğru olan sunulmadan bağlantıyı kesebilir ve
> `authentication failed … server closed the connection` görürsünüz. Doğru
> anahtarı belirtip o sunucu için `IdentitiesOnly yes` ekleyerek çözün:
>
> ```text
> Host web1
>     IdentityFile ~/.ssh/is_ed25519
>     IdentitiesOnly yes
> ```

## Parola korumalı anahtarlar

Parola (passphrase) gizli anahtar dosyasını korur. skry bunu **başlarken**
(arayüz görünmeden önce) bir kez şöyle ele alır:

- Anahtar **ajanınızda zaten yüklüyse** hiçbir şey sorulmaz. skry anahtarı
  açık yarısından tanır.
- Değilse skry sorar:

  ```text
  Enter passphrase for key '/home/siz/.ssh/id_ed25519':
  ```

  Üç deneme hakkınız vardır. Boş cevap o anahtarı atlar.
- Çözülmüş anahtar yalnızca bu çalıştırma için **bellekte** tutulur. Parolanın
  kendisi hemen unutulur; asla yazılmaz ya da loglanmaz.

skry etkileşimli bir terminalde çalışmıyorsa (cron işi, CI hattı, systemd
servisi) soru soramaz. Ajanda olmayan şifreli anahtarlar o zaman atlanır.
Gözetimsiz kullanım için anahtarı bir ajana yükleyin ya da yalnızca bir izleme
hesabına yetkilendirilmiş, parolasız ayrı bir anahtar kullanın.

## Hangi kullanıcı?

Öncelik sırasıyla:

1. Hedefteki kullanıcı: `deploy@web1`
2. `~/.ssh/config`'teki `User`
3. Yerel kullanıcı adınız (`$USER`, Windows'ta `%USERNAME%`)

## Ayrı bir izleme hesabı (ekipler için önerilir)

skry'nin root'a ihtiyacı yoktur. Normal bir hesap yeterlidir; birkaç grup
üyeliği daha fazla veriye erişim sağlar ([İzinler](permissions.md)). Tipik
bir kurulum:

```sh
# her sunucuda
sudo useradd -m -s /bin/sh skry
sudo usermod -aG adm skry          # auth loglarını / journal'ı oku (Debian, Ubuntu)
sudo mkdir -p ~skry/.ssh
echo 'ssh-ed25519 AAAA... izleme' | sudo tee ~skry/.ssh/authorized_keys
sudo chown -R skry: ~skry/.ssh && sudo chmod 700 ~skry/.ssh && sudo chmod 600 ~skry/.ssh/authorized_keys
```

ve kendi bilgisayarınızda:

```text
Host web* db*
    User skry
    IdentityFile ~/.ssh/skry_izleme
    IdentitiesOnly yes
```

Giriş kabuğu gerçek bir kabuk olmalıdır (`/bin/sh` ya da `/bin/bash`).
`nologin`, `rbash` ya da `authorized_keys`'te zorunlu komut (`command=`)
olan hesaplar toplama betiğini çalıştıramaz.

## Ajanı kapatmak

`--no-agent`, skry'nin ajanı yok sayıp yalnızca anahtar dosyalarını
kullanmasını sağlar. Hangi anahtarın tam olarak çalıştığını test etmek için
kullanışlıdır.
