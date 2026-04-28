using System.Security.Cryptography.X509Certificates;
using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public interface ICertificateService
    {
        Task<X509Certificate2> GetOrCreateRootCertificateAsync();
        Task<X509Certificate2> CreateDomainCertificateAsync(string domain);
        Task InstallRootCertificateAsync();
        Task UninstallRootCertificateAsync();
        bool IsRootCertificateInstalled();
    }
}
