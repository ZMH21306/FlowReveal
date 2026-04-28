using System;
using System.IO;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public class CertificateService : ICertificateService
    {
        private const string RootCertName = "FlowReveal Root CA";
        private const string CertStorePath = @"%APPDATA%\FlowReveal\Certificates";

        public async Task<X509Certificate2> GetOrCreateRootCertificateAsync()
        {
            var storePath = Environment.ExpandEnvironmentVariables(CertStorePath);
            Directory.CreateDirectory(storePath);

            var certPath = Path.Combine(storePath, "root.cer");
            var keyPath = Path.Combine(storePath, "root.key");

            if (File.Exists(certPath) && File.Exists(keyPath))
            {
                var certBytes = File.ReadAllBytes(certPath);
                var keyBytes = File.ReadAllBytes(keyPath);
                var cert = new X509Certificate2(certBytes);
                // 这里需要加载私钥，暂时简化实现
                return cert;
            }

            return await CreateRootCertificateAsync();
        }

        public async Task<X509Certificate2> CreateDomainCertificateAsync(string domain)
        {
            var rootCert = await GetOrCreateRootCertificateAsync();
            // 这里将实现动态域名证书生成
            return rootCert;
        }

        public async Task InstallRootCertificateAsync()
        {
            var rootCert = await GetOrCreateRootCertificateAsync();
            using (var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser))
            {
                store.Open(OpenFlags.ReadWrite);
                store.Add(rootCert);
            }
        }

        public async Task UninstallRootCertificateAsync()
        {
            var rootCert = await GetOrCreateRootCertificateAsync();
            using (var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser))
            {
                store.Open(OpenFlags.ReadWrite);
                store.Remove(rootCert);
            }
        }

        public bool IsRootCertificateInstalled()
        {
            // 这里将实现证书安装检查
            return false;
        }

        private async Task<X509Certificate2> CreateRootCertificateAsync()
        {
            using (var rsa = RSA.Create(2048))
            {
                var request = new CertificateRequest(
                    $"CN={RootCertName}",
                    rsa,
                    HashAlgorithmName.SHA256,
                    RSASignaturePadding.Pkcs1
                );

                request.CertificateExtensions.Add(
                    new X509BasicConstraintsExtension(true, true, 0, true)
                );

                var cert = request.CreateSelfSigned(DateTimeOffset.Now, DateTimeOffset.Now.AddYears(10));
                
                // 保存证书和私钥
                var storePath = Environment.ExpandEnvironmentVariables(CertStorePath);
                Directory.CreateDirectory(storePath);

                var certPath = Path.Combine(storePath, "root.cer");
                var keyPath = Path.Combine(storePath, "root.key");

                File.WriteAllBytes(certPath, cert.Export(X509ContentType.Cert));
                // 这里需要保存私钥，暂时简化实现

                return cert;
            }
        }
    }
}
