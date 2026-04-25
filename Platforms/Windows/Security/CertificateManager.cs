using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Platforms.Windows.Security
{
    public class CertificateInfo
    {
        public string Subject { get; set; } = string.Empty;
        public string Thumbprint { get; set; } = string.Empty;
        public DateTime NotBefore { get; set; }
        public DateTime NotAfter { get; set; }
        public bool IsInstalled { get; set; }
        public string StoreLocation { get; set; } = string.Empty;
    }

    public class CertificateManager
    {
        private readonly ILogger<CertificateManager> _logger;
        private readonly string _certificateDirectory;
        private X509Certificate2? _rootCertificate;
        private RSA? _rootKey;

        private const string RootCertSubject = "CN=FlowReveal Root CA, O=FlowReveal, C=CN";
        private const string RootCertFileName = "FlowReveal_RootCA.pfx";
        private const string RootCertPassword = "FlowReveal2024";

        public event EventHandler<CertificateInfo>? CertificateInstalled;
        public event EventHandler<CertificateInfo>? CertificateRemoved;

        public CertificateManager(ILogger<CertificateManager> logger)
        {
            _logger = logger;
            _certificateDirectory = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "FlowReveal", "Certificates");

            if (!Directory.Exists(_certificateDirectory))
            {
                Directory.CreateDirectory(_certificateDirectory);
            }

            _logger.LogInformation("CertificateManager initialized. Certificate directory: {Dir}", _certificateDirectory);
        }

        public X509Certificate2 GetOrCreateRootCertificate()
        {
            if (_rootCertificate != null)
                return _rootCertificate;

            var certPath = Path.Combine(_certificateDirectory, RootCertFileName);

            if (File.Exists(certPath))
            {
                try
                {
                    _rootCertificate = new X509Certificate2(certPath, RootCertPassword, X509KeyStorageFlags.Exportable);
                    _rootKey = _rootCertificate.GetRSAPrivateKey();

                    if (_rootCertificate.NotAfter < DateTime.UtcNow)
                    {
                        _logger.LogWarning("Root certificate expired, generating new one");
                        _rootCertificate.Dispose();
                        _rootCertificate = null;
                        File.Delete(certPath);
                    }
                    else
                    {
                        _logger.LogInformation("Loaded existing root certificate: {Subject} (expires: {Expiry})",
                            _rootCertificate.Subject, _rootCertificate.NotAfter);
                        return _rootCertificate;
                    }
                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "Failed to load existing root certificate, generating new one");
                    _rootCertificate?.Dispose();
                    _rootCertificate = null;
                }
            }

            return GenerateRootCertificate();
        }

        private X509Certificate2 GenerateRootCertificate()
        {
            _logger.LogInformation("Generating new root certificate...");

            _rootKey = RSA.Create(2048);

            var request = new CertificateRequest(
                RootCertSubject,
                _rootKey,
                HashAlgorithmName.SHA256,
                RSASignaturePadding.Pkcs1);

            request.CertificateExtensions.Add(
                new X509BasicConstraintsExtension(true, true, 0, true));

            request.CertificateExtensions.Add(
                new X509KeyUsageExtension(
                    X509KeyUsageFlags.DigitalSignature | X509KeyUsageFlags.CrlSign | X509KeyUsageFlags.KeyCertSign,
                    true));

            request.CertificateExtensions.Add(
                new X509EnhancedKeyUsageExtension(
                    new OidCollection { new Oid("1.3.6.1.5.5.7.3.1") }, true));

            var subjectKeyIdentifier = new X509SubjectKeyIdentifierExtension(request.PublicKey, false);
            request.CertificateExtensions.Add(subjectKeyIdentifier);

            _rootCertificate = request.CreateSelfSigned(
                DateTimeOffset.UtcNow.AddDays(-1),
                DateTimeOffset.UtcNow.AddYears(10));

            var certPath = Path.Combine(_certificateDirectory, RootCertFileName);
            File.WriteAllBytes(certPath, _rootCertificate.Export(X509ContentType.Pfx, RootCertPassword));

            _logger.LogInformation("Root certificate generated and saved: {Path} (expires: {Expiry})",
                certPath, _rootCertificate.NotAfter);

            return _rootCertificate;
        }

        public X509Certificate2 IssueSiteCertificate(string hostname)
        {
            var rootCert = GetOrCreateRootCertificate();
            var rootPrivateKey = _rootKey ?? rootCert.GetRSAPrivateKey();

            _logger.LogDebug("Issuing site certificate for: {Hostname}", hostname);

            using var siteKey = RSA.Create(2048);

            var request = new CertificateRequest(
                $"CN={hostname}",
                siteKey,
                HashAlgorithmName.SHA256,
                RSASignaturePadding.Pkcs1);

            request.CertificateExtensions.Add(
                new X509BasicConstraintsExtension(false, false, 0, false));

            request.CertificateExtensions.Add(
                new X509KeyUsageExtension(X509KeyUsageFlags.DigitalSignature, false));

            request.CertificateExtensions.Add(
                new X509EnhancedKeyUsageExtension(
                    new OidCollection
                    {
                        new Oid("1.3.6.1.5.5.7.3.1"),
                        new Oid("1.3.6.1.5.5.7.3.2")
                    },
                    false));

            var sanBuilder = new SubjectAlternativeNameBuilder();
            sanBuilder.AddDnsName(hostname);
            if (hostname.StartsWith("www.", StringComparison.OrdinalIgnoreCase))
            {
                sanBuilder.AddDnsName(hostname[4..]);
            }
            else
            {
                sanBuilder.AddDnsName($"www.{hostname}");
            }
            request.CertificateExtensions.Add(sanBuilder.Build());

            request.CertificateExtensions.Add(
                X509AuthorityKeyIdentifierExtension.CreateFromCertificate(rootCert, true, false));

            var serialNumber = new byte[16];
            RandomNumberGenerator.Fill(serialNumber);
            serialNumber[0] &= 0x7F;

            var siteCert = request.Create(
                rootCert,
                DateTimeOffset.UtcNow.AddDays(-1),
                DateTimeOffset.UtcNow.AddYears(1),
                serialNumber);

            var siteCertWithKey = siteCert.CopyWithPrivateKey(siteKey);

            _logger.LogDebug("Site certificate issued for: {Hostname}", hostname);
            return siteCertWithKey;
        }

        public bool InstallRootCertificateToTrustedStore()
        {
            var rootCert = GetOrCreateRootCertificate();

            try
            {
                using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
                store.Open(OpenFlags.ReadWrite);

                var existing = store.Certificates.Find(X509FindType.FindBySubjectName, "FlowReveal Root CA", false);
                if (existing.Count > 0)
                {
                    _logger.LogInformation("Root certificate already installed in trusted store");
                    store.Close();
                    return true;
                }

                store.Add(rootCert);
                store.Close();

                _logger.LogInformation("Root certificate installed to trusted store successfully");
                CertificateInstalled?.Invoke(this, new CertificateInfo
                {
                    Subject = rootCert.Subject,
                    Thumbprint = rootCert.Thumbprint,
                    NotBefore = rootCert.NotBefore,
                    NotAfter = rootCert.NotAfter,
                    IsInstalled = true,
                    StoreLocation = "CurrentUser\\Root"
                });

                return true;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to install root certificate to trusted store");
                return false;
            }
        }

        public bool RemoveRootCertificateFromTrustedStore()
        {
            try
            {
                using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
                store.Open(OpenFlags.ReadWrite);

                var certs = store.Certificates.Find(X509FindType.FindBySubjectName, "FlowReveal Root CA", false);
                var removed = false;

                foreach (var cert in certs)
                {
                    store.Remove(cert);
                    _logger.LogInformation("Removed certificate: {Thumbprint}", cert.Thumbprint);
                    removed = true;
                }

                store.Close();

                if (removed)
                {
                    CertificateRemoved?.Invoke(this, new CertificateInfo
                    {
                        Subject = "FlowReveal Root CA",
                        IsInstalled = false
                    });
                }

                return removed;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to remove root certificate from trusted store");
                return false;
            }
        }

        public CertificateInfo? GetRootCertificateInfo()
        {
            try
            {
                var rootCert = GetOrCreateRootCertificate();

                using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
                store.Open(OpenFlags.ReadOnly);
                var existing = store.Certificates.Find(X509FindType.FindByThumbprint, rootCert.Thumbprint, false);
                store.Close();

                return new CertificateInfo
                {
                    Subject = rootCert.Subject,
                    Thumbprint = rootCert.Thumbprint,
                    NotBefore = rootCert.NotBefore,
                    NotAfter = rootCert.NotAfter,
                    IsInstalled = existing.Count > 0,
                    StoreLocation = "CurrentUser\\Root"
                };
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to get root certificate info");
                return null;
            }
        }

        public void Dispose()
        {
            _rootCertificate?.Dispose();
            _rootKey?.Dispose();
        }
    }
}
