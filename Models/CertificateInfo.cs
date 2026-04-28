using System.Security.Cryptography.X509Certificates;

namespace FlowReveal.Models
{
    public class CertificateInfo
    {
        public X509Certificate2 RootCertificate { get; set; }
        public string RootCertificatePath { get; set; }
        public string PrivateKeyPath { get; set; }
        public bool IsInstalled { get; set; }
    }
}
