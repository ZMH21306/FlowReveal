using System;
using System.Security.Cryptography.X509Certificates;

namespace FlowReveal.Services.Certificate;

public class RootCAInstaller
{
    private const string RootCaCommonName = "FlowReveal Root CA";

    public bool IsRootCertificateInstalled()
    {
        try
        {
            using var store = new X509Store(StoreName.Root, StoreLocation.LocalMachine);
            store.Open(OpenFlags.ReadOnly);
            
            var certs = store.Certificates.Find(
                X509FindType.FindBySubjectName,
                RootCaCommonName,
                false
            );

            return certs.Count > 0;
        }
        catch
        {
            return false;
        }
    }

    public bool InstallRootCertificate(X509Certificate2 certificate)
    {
        try
        {
            using var store = new X509Store(StoreName.Root, StoreLocation.LocalMachine);
            store.Open(OpenFlags.ReadWrite);

            var existingCerts = store.Certificates.Find(
                X509FindType.FindBySubjectName,
                RootCaCommonName,
                false
            );

            foreach (var existingCert in existingCerts)
            {
                store.Remove(existingCert);
            }

            store.Add(certificate);
            
            return true;
        }
        catch
        {
            return false;
        }
    }

    public bool UninstallRootCertificate()
    {
        try
        {
            using var store = new X509Store(StoreName.Root, StoreLocation.LocalMachine);
            store.Open(OpenFlags.ReadWrite);

            var certs = store.Certificates.Find(
                X509FindType.FindBySubjectName,
                RootCaCommonName,
                false
            );

            foreach (var cert in certs)
            {
                store.Remove(cert);
            }

            return certs.Count > 0;
        }
        catch
        {
            return false;
        }
    }

    public bool InstallToCurrentUserStore(X509Certificate2 certificate)
    {
        try
        {
            using var store = new X509Store(StoreName.Root, StoreLocation.CurrentUser);
            store.Open(OpenFlags.ReadWrite);

            var existingCerts = store.Certificates.Find(
                X509FindType.FindBySubjectName,
                RootCaCommonName,
                false
            );

            foreach (var existingCert in existingCerts)
            {
                store.Remove(existingCert);
            }

            store.Add(certificate);
            
            return true;
        }
        catch
        {
            return false;
        }
    }
}